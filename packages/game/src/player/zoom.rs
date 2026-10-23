use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

use super::focus::{Aside, Focused, View};
use super::free_look::MODIFIERS;
use crate::state::AppState;

/// How far a trackpad has to scroll for one step.
const STEP_PIXELS: f32 = 40.0;

pub(super) struct ZoomPlugin;

impl Plugin for ZoomPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Zoom::step_on_wheel.run_if(in_state(AppState::Playing)),
        );
    }
}

/// Moves the camera from the view of a focused thing towards its `close`
/// view, in steps, with Ctrl and the mouse wheel. Things without one zoom
/// with the [`Lens`](super::lens::Lens). The zoom stays after Ctrl comes up
/// and for the next time, so the player can read or type while close.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Zoom {
    pub(crate) close: View,
    steps: u8,
    step: u8,
}

impl Zoom {
    pub(crate) const fn to(close: View, steps: u8) -> Self {
        Self {
            close,
            steps,
            step: 0,
        }
    }

    pub(crate) fn zoomed(&self) -> bool {
        self.step > 0
    }

    pub(crate) fn reset(&mut self) {
        self.step = 0;
    }

    /// Zooms in by `steps`, or out if negative.
    pub(crate) fn step_by(&mut self, steps: i32) {
        self.step = (i32::from(self.step) + steps).clamp(0, i32::from(self.steps)) as u8;
    }

    /// `view` moved towards the close view for a target at `target`.
    pub(crate) fn apply(&self, view: Transform, target: &GlobalTransform) -> Transform {
        if self.step == 0 {
            return view;
        }
        let close = self.close.resolve(target);
        let t = f32::from(self.step) / f32::from(self.steps);
        Transform {
            translation: view.translation.lerp(close.translation, t),
            rotation: view.rotation.slerp(close.rotation, t),
            scale: view.scale,
        }
    }

    /// Each notch of a wheel with Ctrl held is one step. A trackpad steps
    /// every [`STEP_PIXELS`]. Nothing zooms while the view is turned aside.
    fn step_on_wheel(
        keys: Res<ButtonInput<KeyCode>>,
        mut wheel: MessageReader<MouseWheel>,
        focused: Res<Focused>,
        mut targets: Query<(&mut Zoom, Option<&Aside>)>,
        mut swipe: Local<f32>,
    ) {
        let held = keys.any_pressed(MODIFIERS);
        let mut steps = 0;
        for event in wheel.read() {
            if !held {
                continue;
            }
            match event.unit {
                MouseScrollUnit::Line => steps += event.y.signum() as i32,
                MouseScrollUnit::Pixel => {
                    if *swipe * event.y < 0.0 {
                        *swipe = 0.0;
                    }
                    *swipe += event.y;
                    if swipe.abs() >= STEP_PIXELS {
                        steps += swipe.signum() as i32;
                        *swipe = 0.0;
                    }
                }
            }
        }
        if !held {
            *swipe = 0.0;
        }
        if steps == 0 {
            return;
        }
        let Some(Ok((mut zoom, aside))) = focused.get().map(|entity| targets.get_mut(entity))
        else {
            return;
        };
        if aside.is_some_and(|aside| aside.open) {
            return;
        }
        let mut stepped = *zoom;
        stepped.step_by(steps);
        zoom.set_if_neq(stepped);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_stay_in_range_and_move_towards_the_close_view() {
        let view = Transform::from_xyz(0.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y);
        let close = Transform::from_xyz(0.0, 0.0, 0.4);
        let mut zoom = Zoom::to(View::Local(close), 2);
        assert_eq!(zoom.apply(view, &GlobalTransform::IDENTITY), view);
        zoom.step_by(5);
        let closest = zoom.apply(view, &GlobalTransform::IDENTITY);
        assert!(closest.translation.distance(close.translation) < 1e-5);
        assert!(closest.rotation.angle_between(close.rotation) < 1e-5);
        zoom.step_by(-1);
        let half = zoom.apply(view, &GlobalTransform::IDENTITY);
        let middle = view.translation.lerp(close.translation, 0.5);
        assert!(half.translation.distance(middle) < 1e-5);
        zoom.step_by(-3);
        assert!(!zoom.zoomed());
    }

    #[test]
    fn a_close_view_follows_the_target() {
        let close = Transform::from_xyz(0.0, 0.0, 0.2);
        let mut zoom = Zoom::to(View::Local(close), 1);
        zoom.step_by(1);
        let target = GlobalTransform::from_xyz(1.0, 0.0, 0.0);
        let pose = zoom.apply(Transform::IDENTITY, &target);
        assert_eq!(pose.translation, Vec3::new(1.0, 0.0, 0.2));
    }
}
