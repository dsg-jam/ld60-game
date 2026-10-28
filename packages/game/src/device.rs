mod board;
mod casing;
mod emulator;
mod handheld;
mod link;
mod name;
mod pick_area;
mod silhouette;

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    picking::{
        events::{PointerDrag, PointerPress},
        pointer::PointerButton,
    },
    prelude::*,
};
use ld60_core::parts::keypad::PressableKey;
use ld60_core::player::camera::PlayerCamera;
use ld60_core::player::focus::{Focus, FocusTarget, Focused, View};
use ld60_core::player::hints::{Hint, Hints, Icon, Input};
use ld60_locale::Message;

pub(crate) use self::casing::{Casing, Header, Turning};
pub(crate) use self::emulator::{Emulator, Stop};
pub(crate) use self::handheld::Handheld;
pub(crate) use self::link::Connected;
pub(crate) use self::name::{BoardLetter, DeviceName};
pub(crate) use self::silhouette::Silhouette;

/// How far a held device lifts off the counter.
const LIFT: f32 = 0.1;
/// How much a held device faces up rather than towards the player.
const TILT_UP: f32 = 0.8;
/// Distance of the player's eyes from a held device.
const VIEW_DISTANCE: f32 = 0.26;
/// Radians of rotation per pixel dragged.
const DRAG_SPEED: f32 = 0.01;
/// How quickly a device moves between the counter and the hand.
const DECAY_RATE: f32 = 10.0;

pub(crate) struct DevicePlugin;

impl Plugin for DevicePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DeviceKey>()
            .add_plugins((
                casing::CasingPlugin,
                emulator::EmulatorPlugin,
                handheld::HandheldPlugin,
            ))
            .add_systems(
                Update,
                (
                    Holdable::frame,
                    Holdable::follow,
                    DeviceButton::send_keys.after(PressableKey::update),
                    DeviceKey::log,
                ),
            )
            .add_systems(OnEnter(Focus::Device), DeviceButton::activate::<true>)
            .add_systems(
                OnExit(Focus::Device),
                (Holdable::reset_spin, DeviceButton::activate::<false>),
            )
            .init_resource::<Turned>()
            .add_observer(Holdable::on_add)
            .add_observer(pick_area::PickArea::on_add)
            .add_observer(Holdable::spin_on_drag)
            .add_observer(Turned::reset);
    }
}

/// A key on a device was pressed or released.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct DeviceKey {
    pub(crate) device: Entity,
    pub(crate) key: char,
    pub(crate) pressed: bool,
}

impl DeviceKey {
    fn log(mut keys: MessageReader<DeviceKey>) {
        for key in keys.read() {
            debug!(
                "device {} key {} {}",
                key.device,
                key.key,
                if key.pressed { "down" } else { "up" }
            );
        }
    }
}

/// How far the pointer was dragged to turn the held device since it was last
/// pressed, in pixels. A click at the end of a turn does nothing else.
#[derive(Resource, Debug, Default)]
pub(crate) struct Turned(f32);

impl Turned {
    /// Further than this, a press and release is a turn, not a click.
    const CLICK: f32 = 4.0;

    pub(crate) fn clicked(&self) -> bool {
        self.0 <= Self::CLICK
    }

    fn reset(_: On<PointerPress>, mut turned: ResMut<Self>) {
        turned.0 = 0.0;
    }
}

/// Something else moves the device, so it neither rests nor follows the
/// player.
#[derive(Component, Debug, Default)]
pub(crate) struct Carried;

/// A device the player can pick up, rotate and put back.
///
/// It rests where it was spawned and can be focused from the seat.
#[derive(Component, Debug, Default)]
#[require(Transform, Visibility)]
pub(crate) struct Holdable {
    rest: Transform,
    /// Rotation applied by dragging, in camera space.
    spin: Quat,
}

impl Holdable {
    fn on_add(
        add: On<Add<Holdable>>,
        mut holdables: Query<(&mut Holdable, &Transform)>,
        mut commands: Commands,
    ) {
        if let Ok((mut holdable, transform)) = holdables.get_mut(add.entity) {
            holdable.rest = *transform;
        }
        commands.entity(add.entity).insert((
            FocusTarget {
                prompt: "click-device",
                focus: Focus::Device,
                // `Holdable::frame` keeps it in front of the seat.
                view: View::World(Transform::IDENTITY),
            },
            Hints(vec![
                Hint {
                    inputs: vec![Input::Icon(Icon::MouseLeft)],
                    action: Message::new("hint-press-keys"),
                },
                Hint {
                    inputs: vec![Input::Drag],
                    action: Message::new("hint-turn-device"),
                },
            ]),
        ));
    }

    /// Pose lifted above its resting place. The top tilts towards the player
    /// but stays in the light from above.
    fn held_pose(&self, seat: Transform) -> Transform {
        let center = self.held_center();
        let towards = (seat.translation - center).normalize() + Vec3::Y * TILT_UP;
        let facing = Transform::from_translation(center + towards).looking_at(center, Vec3::Y);
        let face_up = Quat::from_rotation_x(FRAC_PI_2);
        Transform::from_translation(center).with_rotation(facing.rotation * self.spin * face_up)
    }

    /// Where it rests on the counter.
    pub(crate) fn rest(&self) -> &Transform {
        &self.rest
    }

    /// Turns the resting pose upside down onto the top of a device `height`
    /// tall, or back.
    fn turn_over(&mut self, height: f32) {
        self.rest.translation += self.rest.rotation * Vec3::Y * height;
        self.rest.rotation *= Quat::from_rotation_z(PI);
    }

    fn held_center(&self) -> Vec3 {
        self.rest.translation + Vec3::Y * LIFT
    }

    /// The player leans in from the seat to look at the held device.
    fn view(&self, seat: Transform) -> Transform {
        let center = self.held_center();
        let towards_seat = (seat.translation - center).normalize();
        Transform::from_translation(center + towards_seat * VIEW_DISTANCE)
            .looking_at(center, Vec3::Y)
    }

    fn frame(camera: Single<&PlayerCamera>, mut targets: Query<(&Holdable, &mut FocusTarget)>) {
        for (holdable, mut target) in &mut targets {
            target.view = View::World(holdable.view(camera.seat()));
        }
    }

    pub(crate) fn follow(
        time: Res<Time>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        camera: Single<&PlayerCamera>,
        mut holdables: Query<(Entity, &Holdable, &mut Transform), Without<Carried>>,
    ) {
        let dt = time.delta_secs();
        for (entity, holdable, mut transform) in &mut holdables {
            let held = *state.get() == Focus::Device && focused.get() == Some(entity);
            let goal = if held {
                holdable.held_pose(camera.seat())
            } else {
                holdable.rest
            };
            transform
                .translation
                .smooth_nudge(&goal.translation, DECAY_RATE, dt);
            transform
                .rotation
                .smooth_nudge(&goal.rotation, DECAY_RATE, dt);
        }
    }

    fn spin_on_drag(
        drag: On<PointerDrag>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        mut turned: ResMut<Turned>,
        mut holdables: Query<&mut Holdable>,
    ) {
        if drag.button != PointerButton::Primary
            || *state.get() != Focus::Device
            || focused.get() != Some(drag.entity)
        {
            return;
        }
        if let Ok(mut holdable) = holdables.get_mut(drag.entity) {
            turned.0 += drag.delta.length();
            let delta = drag.delta * DRAG_SPEED;
            holdable.spin =
                Quat::from_rotation_y(delta.x) * Quat::from_rotation_x(delta.y) * holdable.spin;
        }
    }

    fn reset_spin(mut holdables: Query<&mut Holdable>) {
        for mut holdable in &mut holdables {
            holdable.spin = Quat::IDENTITY;
        }
    }
}

/// A physical key on a device. Works with a [`PressableKey`].
#[derive(Component, Debug)]
pub(crate) struct DeviceButton {
    key: char,
    was_pressed: bool,
}

impl DeviceButton {
    pub(crate) fn new(key: char) -> Self {
        Self {
            key,
            was_pressed: false,
        }
    }

    /// Only the buttons of the held device can be pressed.
    fn activate<const ACTIVE: bool>(
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        mut keys: Query<(Entity, &mut PressableKey), With<DeviceButton>>,
    ) {
        for (entity, mut key) in &mut keys {
            key.active = ACTIVE
                && parents
                    .iter_ancestors(entity)
                    .any(|ancestor| focused.get() == Some(ancestor));
        }
    }

    fn send_keys(
        parents: Query<&ChildOf>,
        holdables: Query<(), With<Holdable>>,
        mut buttons: Query<(Entity, &mut DeviceButton, &PressableKey)>,
        mut keys: MessageWriter<DeviceKey>,
    ) {
        for (entity, mut button, key) in &mut buttons {
            if key.is_pressed() == button.was_pressed {
                continue;
            }
            button.was_pressed = key.is_pressed();
            if let Some(device) = parents
                .iter_ancestors(entity)
                .find(|ancestor| holdables.contains(*ancestor))
            {
                keys.write(DeviceKey {
                    device,
                    key: button.key,
                    pressed: key.is_pressed(),
                });
            }
        }
    }
}
