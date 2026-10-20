use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

use super::focus::{Aside, Focused, View};
use super::free_look::{FreeLook, MODIFIERS};
use crate::menu::AppState;

/// How far a trackpad has to scroll for one step.
const STEP_PIXELS: f32 = 40.0;

pub(super) struct ZoomPlugin;

impl Plugin for ZoomPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Zoom::step_on_wheel.run_if(in_state(AppState::Playing)),
                Zoom::reset_unfocused.run_if(resource_changed::<Focused>),
            ),
        );
    }
}

/// Lets the player lean in on a focused thing with Ctrl and the mouse
/// wheel, in steps from its view up to the `close` one. The zoom stays after
/// Ctrl comes up, so the player can read or type while close.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Zoom {
    pub(crate) close: Closer,
    steps: u8,
    step: u8,
    /// Whether the zoom stays for the next time the player comes back.
    keep: bool,
}

/// Where a [`Zoom`] ends up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Closer {
    View(View),
    /// Meters ahead of the focused view.
    Ahead(f32),
}

impl Zoom {
    pub(crate) const fn ahead(meters: f32, steps: u8) -> Self {
        Self::to(Closer::Ahead(meters), steps)
    }

    pub(crate) const fn to(close: Closer, steps: u8) -> Self {
        Self {
            close,
            steps,
            step: 0,
            keep: false,
        }
    }

    pub(crate) const fn kept(self) -> Self {
        Self { keep: true, ..self }
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
        let close = match self.close {
            Closer::View(close) => close.resolve(target),
            Closer::Ahead(meters) => {
                view.with_translation(view.translation + view.forward() * meters)
            }
        };
        let t = f32::from(self.step) / f32::from(self.steps);
        Transform {
            translation: view.translation.lerp(close.translation, t),
            rotation: view.rotation.slerp(close.rotation, t),
            scale: view.scale,
        }
    }

    /// Each notch of a wheel with Ctrl held is one step. A trackpad steps
    /// every [`STEP_PIXELS`]. Nothing zooms while the view is turned aside.
    /// In free look the wheel zooms the [`Lens`](super::lens::Lens) instead.
    fn step_on_wheel(
        keys: Res<ButtonInput<KeyCode>>,
        free_look: Res<FreeLook>,
        mut wheel: MessageReader<MouseWheel>,
        focused: Res<Focused>,
        mut targets: Query<(&mut Zoom, Option<&Aside>)>,
        mut swipe: Local<f32>,
    ) {
        let held = keys.any_pressed(MODIFIERS) && !free_look.looking();
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

    fn reset_unfocused(focused: Res<Focused>, mut zooms: Query<(Entity, &mut Zoom)>) {
        for (entity, mut zoom) in &mut zooms {
            if !zoom.keep && zoom.zoomed() && focused.get() != Some(entity) {
                zoom.reset();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_stay_in_range_and_move_ahead() {
        let view = Transform::from_xyz(0.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y);
        let mut zoom = Zoom::ahead(0.4, 2);
        assert_eq!(zoom.apply(view, &GlobalTransform::IDENTITY), view);
        zoom.step_by(5);
        let close = zoom.apply(view, &GlobalTransform::IDENTITY);
        assert!((close.translation.distance(view.translation) - 0.4).abs() < 1e-5);
        assert!(close.rotation.angle_between(view.rotation) < 1e-5);
        zoom.step_by(-1);
        let half = zoom.apply(view, &GlobalTransform::IDENTITY);
        assert!((half.translation.distance(view.translation) - 0.2).abs() < 1e-5);
        zoom.step_by(-3);
        assert!(!zoom.zoomed());
    }

    #[test]
    fn a_close_view_follows_the_target() {
        let close = Transform::from_xyz(0.0, 0.0, 0.2);
        let mut zoom = Zoom::to(Closer::View(View::Local(close)), 1);
        zoom.step_by(1);
        let target = GlobalTransform::from_xyz(1.0, 0.0, 0.0);
        let pose = zoom.apply(Transform::IDENTITY, &target);
        assert_eq!(pose.translation, Vec3::new(1.0, 0.0, 0.2));
    }
}
