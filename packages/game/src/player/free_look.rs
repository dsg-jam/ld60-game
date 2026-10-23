use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};

use super::focus::Focus;
use super::lens::Lens;
use super::look::SENSITIVITY;
use crate::settings::Settings;
use crate::state::AppState;

/// Held to look around while focused, and with the wheel to zoom.
pub(crate) const MODIFIERS: [KeyCode; 2] = [KeyCode::ControlLeft, KeyCode::ControlRight];
/// How far the head turns from the focused view, in radians. Enough to see
/// the pinned sheets from the computer.
const YAW_LIMIT: f32 = 1.0;
const PITCH_LIMIT: f32 = 0.5;

pub(super) struct FreeLookPlugin;

impl Plugin for FreeLookPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FreeLook>()
            .add_systems(Update, FreeLook::update);
    }
}

/// Turns the head away from the focused view while Ctrl is held and the
/// mouse moves. Letting go of Ctrl looks back.
///
/// Ctrl with another key is a shortcut, like undo in the editor, so the
/// head stays until Ctrl comes up again.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FreeLook {
    yaw: f32,
    pitch: f32,
    phase: Phase,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Off,
    /// Ctrl is down, but the mouse hasn't moved yet.
    Ready,
    Looking,
    Shortcut,
}

impl FreeLook {
    pub(crate) fn looking(&self) -> bool {
        self.phase == Phase::Looking
    }

    /// `view` with the head turned.
    ///
    /// Sideways the head turns about the world's up in level views and about
    /// the view's own up in views looking down, blended by how steep the view
    /// is. So the mouse always turns towards the side of the screen, and
    /// level views keep the horizon level.
    pub(crate) fn turn(&self, view: Transform) -> Transform {
        let steep = view.forward().y.powi(2);
        let up = Vec3::Y.lerp(*view.up(), steep).normalize_or(Vec3::Y);
        view.with_rotation(
            Quat::from_axis_angle(up, self.yaw) * view.rotation * Quat::from_rotation_x(self.pitch),
        )
    }

    pub(super) fn update(
        app_state: Res<State<AppState>>,
        focus: Res<State<Focus>>,
        keys: Res<ButtonInput<KeyCode>>,
        motion: Res<AccumulatedMouseMotion>,
        settings: Res<Settings>,
        lens: Res<Lens>,
        mut free_look: ResMut<FreeLook>,
    ) {
        let focused = *app_state.get() == AppState::Playing && *focus.get() != Focus::Desk;
        let held = focused && keys.any_pressed(MODIFIERS);
        let shortcut = keys.get_just_pressed().any(|key| !MODIFIERS.contains(key));
        let delta = motion.delta * SENSITIVITY * settings.sensitivity() / lens.zoom();
        let mut next = *free_look;
        next.step(held, shortcut, delta);
        free_look.set_if_neq(next);
    }

    /// Follows Ctrl being `held`, another key just pressed for a
    /// `shortcut` and the mouse moving by `delta` radians.
    fn step(&mut self, held: bool, shortcut: bool, delta: Vec2) {
        if !held {
            *self = Self::default();
            return;
        }
        self.phase = match self.phase {
            Phase::Off | Phase::Ready if shortcut => Phase::Shortcut,
            Phase::Off | Phase::Ready if delta != Vec2::ZERO => Phase::Looking,
            Phase::Off => Phase::Ready,
            phase => phase,
        };
        if self.looking() {
            self.yaw = (self.yaw - delta.x).clamp(-YAW_LIMIT, YAW_LIMIT);
            self.pitch = (self.pitch - delta.y).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_while_held_and_back_on_release() {
        let mut look = FreeLook::default();
        look.step(true, false, Vec2::ZERO);
        assert!(!look.looking());
        look.step(true, false, Vec2::new(0.2, -0.1));
        assert!(look.looking());
        assert_eq!((look.yaw, look.pitch), (-0.2, 0.1));
        look.step(true, false, Vec2::new(-10.0, 10.0));
        assert_eq!((look.yaw, look.pitch), (YAW_LIMIT, -PITCH_LIMIT));
        look.step(false, false, Vec2::ZERO);
        assert_eq!(look, FreeLook::default());
    }

    #[test]
    fn a_shortcut_keeps_the_head_still() {
        let mut look = FreeLook::default();
        look.step(true, false, Vec2::ZERO);
        look.step(true, true, Vec2::ZERO);
        look.step(true, false, Vec2::new(0.2, 0.0));
        assert!(!look.looking());
        assert_eq!(look.yaw, 0.0);
        look.step(false, false, Vec2::ZERO);
        look.step(true, false, Vec2::new(0.2, 0.0));
        assert!(look.looking());
    }

    #[test]
    fn keys_while_looking_go_on_looking() {
        let mut look = FreeLook::default();
        look.step(true, false, Vec2::new(0.1, 0.0));
        look.step(true, true, Vec2::new(0.1, 0.0));
        assert!(look.looking());
        assert!((look.yaw + 0.2).abs() < 1e-6);
    }

    fn turned(view: Transform, yaw: f32, pitch: f32) -> Transform {
        FreeLook {
            yaw,
            pitch,
            phase: Phase::Looking,
        }
        .turn(view)
    }

    #[test]
    fn turning_moves_towards_the_side_and_top_of_the_screen() {
        let level = Transform::from_xyz(0.0, 1.0, 1.0).looking_to(Vec3::NEG_Z, Vec3::Y);
        let pitched =
            Transform::from_xyz(0.0, 1.0, 1.0).looking_to(Vec3::new(0.0, -0.3, -1.0), Vec3::Y);
        let down = Transform::from_xyz(0.0, 1.0, 0.0).looking_to(Vec3::NEG_Y, Vec3::NEG_Z);
        for view in [level, pitched, down] {
            let left = turned(view, 0.4, 0.0);
            assert!(left.forward().dot(*view.left()) > 0.35, "{view:?}");
            assert!(left.forward().dot(*view.up()).abs() < 0.05, "{view:?}");
            let up = turned(view, 0.0, 0.3);
            assert!(up.forward().dot(*view.up()) > 0.25, "{view:?}");
            assert!(up.forward().dot(*view.left()).abs() < 1e-5, "{view:?}");
        }
        assert!(turned(level, 0.8, 0.0).right().y.abs() < 1e-5);
        assert!(turned(pitched, 0.8, 0.0).right().y.abs() < 0.03);
        let spin = turned(down, 0.8, 0.0).up().angle_between(*down.up());
        assert!(spin < 1e-4, "{spin}");
    }

    #[test]
    fn turning_keeps_the_eye() {
        let view = Transform::from_xyz(0.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y);
        let look = FreeLook {
            yaw: 0.5,
            pitch: 0.0,
            phase: Phase::Looking,
        };
        let turned = look.turn(view);
        assert_eq!(turned.translation, view.translation);
        assert!(turned.rotation.angle_between(view.rotation) > 0.4);
    }
}
