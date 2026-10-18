mod summa6;

use std::f32::consts::FRAC_PI_2;

use bevy::{
    picking::{events::PointerDrag, pointer::PointerButton},
    prelude::*,
};

pub(crate) use self::summa6::Summa6;
use crate::parts::keypad::PressableKey;
use crate::player::camera::PlayerCamera;
use crate::player::focus::{Focus, FocusTarget, Focused, View};

/// Distance of a held device from the player's eyes.
const HOLD_DISTANCE: f32 = 0.24;
/// Radians of rotation per pixel dragged.
const DRAG_SPEED: f32 = 0.01;
/// How quickly a device moves between the desk and the hand.
const DECAY_RATE: f32 = 10.0;

pub(crate) struct DevicePlugin;

impl Plugin for DevicePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DeviceKey>()
            .add_plugins(summa6::Summa6Plugin)
            .add_systems(
                Update,
                (
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
            .add_observer(Holdable::on_add)
            .add_observer(Holdable::spin_on_drag);
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

/// A device the player can pick up, rotate and put back.
///
/// It rests where it was spawned and can be focused from the desk.
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
        commands.entity(add.entity).insert(FocusTarget {
            focus: Focus::Device,
            view: View::Seated,
        });
    }

    /// Pose in front of the seated player, with the top facing them.
    fn held_pose(&self, seat: Transform) -> Transform {
        let face_player = Quat::from_rotation_x(FRAC_PI_2);
        Transform::from_translation(seat.translation + seat.forward() * HOLD_DISTANCE)
            .with_rotation(seat.rotation * self.spin * face_player)
    }

    fn follow(
        time: Res<Time>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        camera: Single<&PlayerCamera>,
        mut holdables: Query<(Entity, &Holdable, &mut Transform)>,
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
        mut holdables: Query<&mut Holdable>,
    ) {
        if drag.button != PointerButton::Primary
            || *state.get() != Focus::Device
            || focused.get() != Some(drag.entity)
        {
            return;
        }
        if let Ok(mut holdable) = holdables.get_mut(drag.entity) {
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
