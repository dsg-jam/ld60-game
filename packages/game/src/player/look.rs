use bevy::{
    camera::RenderTarget,
    input::mouse::AccumulatedMouseMotion,
    picking::{
        PickingSystems,
        input::mouse_pick_events,
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowRef},
};

use super::focus::Focus;

const SENSITIVITY: f32 = 0.0025;
const MAX_YAW: f32 = 1.2;
const MAX_PITCH: f32 = 0.7;
const CROSSHAIR_SIZE: f32 = 6.0;

pub(super) struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LookAngles>()
            .add_systems(Startup, Crosshair::spawn)
            .add_systems(
                Update,
                (
                    update_cursor,
                    LookAngles::update.run_if(in_state(Focus::Desk)),
                    Crosshair::update,
                ),
            )
            .add_systems(
                First,
                center_pointer
                    .in_set(PickingSystems::Input)
                    .after(mouse_pick_events),
            );
    }
}

/// Head rotation of the seated player, relative to looking straight ahead.
#[derive(Resource, Default, Debug)]
pub(crate) struct LookAngles {
    yaw: f32,
    pitch: f32,
}

impl LookAngles {
    /// Turns the head of `seat`: yaw around the world up axis, pitch around
    /// the camera's own right axis.
    pub(crate) fn apply(&self, seat: Transform) -> Transform {
        let yaw = Quat::from_rotation_y(self.yaw);
        let pitch = Quat::from_axis_angle(*seat.right(), self.pitch);
        seat.with_rotation(yaw * pitch * seat.rotation)
    }

    fn update(
        motion: Res<AccumulatedMouseMotion>,
        cursor: Single<&CursorOptions, With<PrimaryWindow>>,
        mut angles: ResMut<LookAngles>,
    ) {
        if cursor.grab_mode != CursorGrabMode::Locked {
            return;
        }
        let delta = motion.delta * SENSITIVITY;
        angles.yaw = (angles.yaw - delta.x).clamp(-MAX_YAW, MAX_YAW);
        angles.pitch = (angles.pitch - delta.y).clamp(-MAX_PITCH, MAX_PITCH);
    }
}

/// Dot in the middle of the view that marks what a click hits.
#[derive(Component)]
struct Crosshair;

impl Crosshair {
    fn spawn(mut commands: Commands) {
        commands.spawn((
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
            children![(
                Crosshair,
                Node {
                    width: px(CROSSHAIR_SIZE),
                    height: px(CROSSHAIR_SIZE),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 0.98, 0.9, 0.8)),
                Pickable::IGNORE,
                Visibility::Hidden,
            )],
        ));
    }

    fn update(
        state: Res<State<Focus>>,
        cursor: Single<&CursorOptions, With<PrimaryWindow>>,
        mut crosshair: Single<&mut Visibility, With<Crosshair>>,
    ) {
        let locked = cursor.grab_mode == CursorGrabMode::Locked;
        **crosshair = if locked && *state.get() == Focus::Desk {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Locks the cursor while looking around and frees it otherwise.
///
/// Escape always frees the cursor. Browsers only allow the lock after a
/// click, so a click in the desk view locks it again.
fn update_cursor(
    state: Res<State<Focus>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let desk = *state.get() == Focus::Desk;
    let free = keys.just_pressed(KeyCode::Escape) || (!desk && state.is_changed());
    let lock = desk && (state.is_changed() || mouse.just_pressed(MouseButton::Left));
    if free {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else if lock {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Keeps the mouse pointer in the middle of the window while the cursor is
/// locked, so picking hits what the crosshair shows.
fn center_pointer(
    state: Res<State<Focus>>,
    window: Single<(Entity, &Window, &CursorOptions), With<PrimaryWindow>>,
    mut inputs: MessageWriter<PointerInput>,
) {
    let (entity, window, cursor) = *window;
    if *state.get() != Focus::Desk || cursor.grab_mode != CursorGrabMode::Locked {
        return;
    }
    let Some(target) = RenderTarget::Window(WindowRef::Entity(entity)).normalize(Some(entity))
    else {
        return;
    };
    inputs.write(PointerInput::new(
        PointerId::Mouse,
        Location {
            target,
            position: window.size() / 2.0,
        },
        PointerAction::Move { delta: Vec2::ZERO },
    ));
}
