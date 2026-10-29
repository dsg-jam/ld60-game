use std::f32::consts::{PI, TAU};
use std::mem;

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
    window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowFocused, WindowRef},
};

use super::{camera::PlayerCamera, focus::Focus, free_look::FreeLook, lens::Lens};
use crate::settings::Settings;
use crate::state::AppState;

pub(super) const SENSITIVITY: f32 = 0.0025;
/// Just short of looking straight up or down, in radians.
const MAX_PITCH: f32 = 1.5;

pub(super) struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LookAngles>()
            .init_resource::<CursorLock>()
            .add_systems(
                Update,
                (LookAngles::update, reshow_cursor, update_cursor)
                    .chain()
                    .after(FreeLook::update),
            )
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

/// Leaves the system mouse to the user while a tool drives the game: the
/// cursor never locks and losing the window focus does not pause.
#[derive(Resource, Debug)]
pub struct KeepMouse;

/// Head rotation of the seated player, relative to looking straight ahead.
#[derive(Resource, Default, Debug)]
pub struct LookAngles {
    yaw: f32,
    pitch: f32,
}

impl LookAngles {
    /// Turns the head of `seat` by adding the look angles to its own yaw and
    /// pitch, without roll.
    pub fn apply(&self, seat: Transform) -> Transform {
        let (yaw, pitch, _) = seat.rotation.to_euler(EulerRot::YXZ);
        seat.with_rotation(Quat::from_euler(
            EulerRot::YXZ,
            yaw + self.yaw,
            pitch + self.pitch,
            0.0,
        ))
    }

    /// The look angles that turn `seat` to `rotation`.
    fn facing(seat: Transform, rotation: Quat) -> Self {
        let (seat_yaw, seat_pitch, _) = seat.rotation.to_euler(EulerRot::YXZ);
        let (yaw, pitch, _) = rotation.to_euler(EulerRot::YXZ);
        Self {
            yaw: (yaw - seat_yaw + PI).rem_euclid(TAU) - PI,
            pitch: pitch - seat_pitch,
        }
    }

    /// Turns freely, but never past looking straight up or down.
    ///
    /// Moving the mouse while the camera turns on its own takes over from
    /// where it looks. The [`Lens`] zoom slows the turn down with it.
    pub(super) fn update(
        gaze: Gaze,
        motion: Res<AccumulatedMouseMotion>,
        settings: Res<Settings>,
        lens: Res<Lens>,
        camera: Single<(&mut PlayerCamera, &Transform)>,
        mut angles: ResMut<LookAngles>,
    ) {
        let (mut camera, transform) = camera.into_inner();
        if !gaze.looking_around() || motion.delta == Vec2::ZERO || !camera.steerable() {
            return;
        }
        let seat = camera.seat();
        if camera.take_over() {
            *angles = Self::facing(seat, transform.rotation);
        }
        let delta = motion.delta * SENSITIVITY * settings.sensitivity() / lens.zoom();
        let (_, seat_pitch, _) = seat.rotation.to_euler(EulerRot::YXZ);
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
pub struct CursorLock {
    locked: bool,
    lost: bool,
}

impl CursorLock {
    /// Whether the lock went away this frame while the game still wants it.
    pub fn lost(&self) -> bool {
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
    free_look: Res<'w, FreeLook>,
}

impl Gaze<'_> {
    /// Turning the head with the mouse, in the desk view.
    pub(super) fn looking_around(&self) -> bool {
        self.at_desk() && self.lock.locked
    }

    /// In the desk view, whether the cursor is locked yet or not.
    pub(super) fn at_desk(&self) -> bool {
        self.playing() && *self.focus.get() == Focus::Desk
    }

    /// Focused on something, with the cursor free unless
    /// [`Gaze::free_looking`].
    pub(super) fn focused(&self) -> bool {
        self.playing() && *self.focus.get() != Focus::Desk
    }

    /// Turning the head with the mouse while focused. Browsers may refuse
    /// the lock without a recent click, so this doesn't wait for it.
    pub(super) fn free_looking(&self) -> bool {
        self.focused() && self.free_look.looking()
    }

    /// Looking around in the desk view or while focused, with the crosshair
    /// in the middle.
    pub(super) fn steering(&self) -> bool {
        self.looking_around() || self.free_looking()
    }

    fn playing(&self) -> bool {
        *self.app_state.get() == AppState::Playing
    }
}

/// Locks the cursor while looking around, in the desk view or with
/// [`FreeLook`], and frees it otherwise.
///
/// Browsers only allow the lock after a click, so a click in the desk view
/// locks it again. Until a browser grants the lock, the pointer stays
/// visible there to click with. In free look it stays hidden either way.
#[expect(clippy::too_many_arguments, reason = "Bevy system")]
fn update_cursor(
    keep_mouse: Option<Res<KeepMouse>>,
    app_state: Res<State<AppState>>,
    focus: Res<State<Focus>>,
    free_look: Res<FreeLook>,
    mouse: Res<ButtonInput<MouseButton>>,
    held: Res<CursorLock>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut was_looking: Local<bool>,
) {
    let playing = *app_state.get() == AppState::Playing;
    let desk = playing && *focus.get() == Focus::Desk;
    let looking = playing && free_look.looking();
    let changed = app_state.is_changed() || focus.is_changed() || looking != *was_looking;
    *was_looking = looking;
    let wanted = (desk || looking) && keep_mouse.is_none();
    let free = !wanted && changed;
    let lock = wanted && (changed || (desk && mouse.just_pressed(MouseButton::Left)));
    if free {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else if lock {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    } else if cursor.grab_mode == CursorGrabMode::Locked {
        let visible = desk && !held.locked;
        if cursor.visible != visible {
            cursor.visible = visible;
        }
    }
}

/// Hides and shows a free cursor again when the window comes back into focus.
///
/// macOS only applies a change to the cursor while the window has focus. A
/// cursor freed while the window was in the background stays hidden
/// otherwise.
fn reshow_cursor(
    mut events: MessageReader<WindowFocused>,
    cursor: Single<(Entity, &mut CursorOptions), With<PrimaryWindow>>,
    mut hidden: Local<bool>,
) {
    let (window, mut cursor) = cursor.into_inner();
    let back = events
        .read()
        .any(|event| event.focused && event.window == window);
    if mem::take(&mut *hidden) {
        if cursor.grab_mode == CursorGrabMode::None {
            cursor.visible = true;
        }
    } else if back && cursor.visible {
        cursor.visible = false;
        *hidden = true;
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
    if !gaze.steering() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facing_finds_the_angles_of_a_view() {
        let seat = Transform::from_xyz(0.0, 1.5, 0.7).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y);
        let angles = LookAngles {
            yaw: 2.5,
            pitch: 0.3,
        };
        let found = LookAngles::facing(seat, angles.apply(seat).rotation);
        assert!((found.yaw - angles.yaw).abs() < 1e-4, "{found:?}");
        assert!((found.pitch - angles.pitch).abs() < 1e-4, "{found:?}");
    }
}
