use bevy::{
    camera::RenderTarget,
    ecs::system::SystemParam,
    input::mouse::AccumulatedMouseMotion,
    picking::{
        PickingSystems,
        input::mouse_pick_events,
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowRef},
};

use super::{camera::PlayerCamera, focus::Focus};
use crate::menu::{AppState, Settings};

const SENSITIVITY: f32 = 0.0025;
/// Just short of looking straight up or down, in radians.
const MAX_PITCH: f32 = 1.5;

pub(super) struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LookAngles>()
            .init_resource::<CursorLock>()
            .add_systems(Update, (LookAngles::update, update_cursor).chain())
            .add_systems(
                First,
                (
                    CursorLock::update.before(PickingSystems::Input),
                    center_pointer
                        .in_set(PickingSystems::Input)
                        .after(mouse_pick_events),
                ),
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
    /// Turns the head of `seat` by adding the look angles to its own yaw and
    /// pitch, without roll.
    pub(crate) fn apply(&self, seat: Transform) -> Transform {
        let (yaw, pitch, _) = seat.rotation.to_euler(EulerRot::YXZ);
        seat.with_rotation(Quat::from_euler(
            EulerRot::YXZ,
            yaw + self.yaw,
            pitch + self.pitch,
            0.0,
        ))
    }

    /// Turns freely, but never past looking straight up or down.
    fn update(
        gaze: Gaze,
        motion: Res<AccumulatedMouseMotion>,
        settings: Res<Settings>,
        camera: Single<&PlayerCamera>,
        mut angles: ResMut<LookAngles>,
    ) {
        if !gaze.looking_around() {
            return;
        }
        let delta = motion.delta * SENSITIVITY * settings.sensitivity();
        let (_, seat_pitch, _) = camera.seat().rotation.to_euler(EulerRot::YXZ);
        angles.yaw -= delta.x;
        angles.pitch =
            (angles.pitch - delta.y).clamp(-MAX_PITCH - seat_pitch, MAX_PITCH - seat_pitch);
    }
}

/// Whether the cursor is locked.
///
/// Browsers drop the lock on their own, for example on Escape, so on the web
/// this follows the browser and not only [`CursorOptions::grab_mode`].
#[derive(Resource, Default, Debug)]
pub(crate) struct CursorLock {
    locked: bool,
    lost: bool,
}

impl CursorLock {
    /// Whether the lock went away this frame while the game still wants it.
    pub(crate) fn lost(&self) -> bool {
        self.lost
    }

    fn update(cursor: Single<&CursorOptions, With<PrimaryWindow>>, mut lock: ResMut<CursorLock>) {
        let wanted = cursor.grab_mode == CursorGrabMode::Locked;
        let locked = wanted && Self::held_by_browser();
        lock.lost = wanted && lock.locked && !locked;
        lock.locked = locked;
    }

    #[cfg(not(target_arch = "wasm32"))]
    const fn held_by_browser() -> bool {
        true
    }

    #[cfg(target_arch = "wasm32")]
    fn held_by_browser() -> bool {
        web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.pointer_lock_element())
            .is_some()
    }
}

/// What the player looks at, by game state, focus and cursor lock.
#[derive(SystemParam)]
pub(super) struct Gaze<'w> {
    app_state: Res<'w, State<AppState>>,
    focus: Res<'w, State<Focus>>,
    lock: Res<'w, CursorLock>,
}

impl Gaze<'_> {
    /// Turning the head with the mouse, in the desk view.
    pub(super) fn looking_around(&self) -> bool {
        self.playing() && *self.focus.get() == Focus::Desk && self.lock.locked
    }

    /// Focused on something, with the cursor free.
    pub(super) fn focused(&self) -> bool {
        self.playing() && *self.focus.get() != Focus::Desk
    }

    fn playing(&self) -> bool {
        *self.app_state.get() == AppState::Playing
    }
}

/// Locks the cursor while looking around and frees it otherwise.
///
/// Browsers only allow the lock after a click, so a click in the desk view
/// locks it again. Until a browser grants the lock, the pointer stays
/// visible to click with.
fn update_cursor(
    app_state: Res<State<AppState>>,
    focus: Res<State<Focus>>,
    mouse: Res<ButtonInput<MouseButton>>,
    held: Res<CursorLock>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let playing = *app_state.get() == AppState::Playing;
    let desk = playing && *focus.get() == Focus::Desk;
    let changed = app_state.is_changed() || focus.is_changed();
    let free = !desk && changed;
    let lock = desk && (changed || mouse.just_pressed(MouseButton::Left));
    if free {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else if lock {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    } else if cursor.grab_mode == CursorGrabMode::Locked && cursor.visible == held.locked {
        cursor.visible = !held.locked;
    }
}

/// Keeps the mouse pointer in the middle of the window while the cursor is
/// locked, so picking hits what the crosshair shows.
fn center_pointer(
    gaze: Gaze,
    window: Single<(Entity, &Window), With<PrimaryWindow>>,
    mut inputs: MessageWriter<PointerInput>,
) {
    let (entity, window) = *window;
    if !gaze.looking_around() {
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
