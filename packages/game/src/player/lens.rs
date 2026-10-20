use std::f32::consts::FRAC_PI_4;

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

use super::camera::PlayerCamera;
use super::focus::Focused;
use super::free_look::{FreeLook, MODIFIERS};
use super::look::Gaze;
use super::zoom::Zoom;
use crate::menu::AppState;

/// Vertical field of view of the player camera, in radians, without the
/// [`Lens`] zoom.
pub(crate) const FOV: f32 = FRAC_PI_4;
/// How much larger the [`Lens`] shows things at most.
const MAX_ZOOM: f32 = 3.0;
/// How much larger each wheel notch shows things.
const NOTCH: f32 = 1.25;
/// How far a trackpad scrolls for one notch.
const NOTCH_PIXELS: f32 = 40.0;
/// How quickly the zoom follows the wheel.
const DECAY_RATE: f32 = 12.0;

pub(super) struct LensPlugin;

impl Plugin for LensPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lens>().add_systems(
            Update,
            (
                Lens::reset_on_change.run_if(not(in_state(AppState::Paused))),
                Lens::zoom_on_wheel.run_if(in_state(AppState::Playing)),
                Lens::apply,
            )
                .chain()
                .after(FreeLook::update),
        );
    }
}

/// Narrows the field of view with Ctrl and the wheel, like squinting at
/// something. It works seated at the desk and in every focus view without a
/// [`Zoom`] of its own, also while looking around with [`FreeLook`].
///
/// The zoom stays until the player focuses on something else or gets up,
/// also over a pause.
#[derive(Resource, Debug, Default)]
pub(crate) struct Lens {
    /// The zoom the wheel asks for, as the power of [`NOTCH`].
    notches: f32,
    /// The zoom shown, as the power of [`NOTCH`].
    shown: f32,
    holder: Holder,
}

/// What the [`Lens`] zoom belongs to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Holder {
    #[default]
    None,
    Desk,
    Focus(Entity),
}

impl Lens {
    pub(crate) fn zoomed(&self) -> bool {
        self.notches > 0.0
    }

    pub(crate) fn reset(&mut self) {
        self.notches = 0.0;
    }

    /// How much larger things look.
    pub(crate) fn zoom(&self) -> f32 {
        NOTCH.powf(self.shown)
    }

    /// The vertical field of view for the zoom.
    fn fov(&self) -> f32 {
        2.0 * ((FOV / 2.0).tan() / self.zoom()).atan()
    }

    /// Zooms in by `notches`, or out if negative.
    pub(super) fn step(&mut self, notches: f32) {
        self.notches = (self.notches + notches).clamp(0.0, MAX_ZOOM.log(NOTCH));
    }

    /// The zoom starts over whenever it would belong to something else.
    fn reset_on_change(
        gaze: Gaze,
        focused: Res<Focused>,
        zooms: Query<(), With<Zoom>>,
        camera: Single<(&PlayerCamera, &Transform)>,
        mut lens: ResMut<Lens>,
    ) {
        let (camera, transform) = *camera;
        let holder = match focused.get() {
            None if gaze.at_desk() && camera.seated(transform) => Holder::Desk,
            Some(entity) if gaze.focused() && !zooms.contains(entity) => Holder::Focus(entity),
            _ => Holder::None,
        };
        if lens.holder != holder {
            lens.holder = holder;
            lens.reset();
        }
    }

    fn zoom_on_wheel(
        keys: Res<ButtonInput<KeyCode>>,
        mut wheel: MessageReader<MouseWheel>,
        mut lens: ResMut<Lens>,
    ) {
        let held = keys.any_pressed(MODIFIERS) && lens.holder != Holder::None;
        let notches: f32 = wheel
            .read()
            .filter(|_| held)
            .map(|event| match event.unit {
                MouseScrollUnit::Line => event.y,
                MouseScrollUnit::Pixel => event.y / NOTCH_PIXELS,
            })
            .sum();
        if notches != 0.0 {
            lens.step(notches);
        }
    }

    fn apply(
        time: Res<Time>,
        mut lens: ResMut<Lens>,
        mut projection: Single<&mut Projection, With<PlayerCamera>>,
    ) {
        if lens.shown != lens.notches {
            let notches = lens.notches;
            lens.shown
                .smooth_nudge(&notches, DECAY_RATE, time.delta_secs());
            if (lens.shown - notches).abs() < 1e-3 {
                lens.shown = notches;
            }
        }
        let fov = lens.fov();
        if let Projection::Perspective(perspective) = &mut **projection
            && perspective.fov != fov
        {
            perspective.fov = fov;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_zoom_stays_in_range_and_magnifies() {
        let mut lens = Lens::default();
        assert_eq!(lens.fov(), FOV);
        lens.step(-2.0);
        assert_eq!(lens.notches, 0.0);
        lens.step(100.0);
        lens.shown = lens.notches;
        assert!((lens.zoom() - MAX_ZOOM).abs() < 1e-4);
        let magnified = (FOV / 2.0).tan() / (lens.fov() / 2.0).tan();
        assert!((magnified - MAX_ZOOM).abs() < 1e-4);
    }
}
