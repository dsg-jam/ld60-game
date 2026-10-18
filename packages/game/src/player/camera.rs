use bevy::{core_pipeline::tonemapping::DebandDither, prelude::*};

use super::focus::{FocusTarget, Focused, View};
use super::look::LookAngles;

/// How quickly the camera moves to a new view.
const DECAY_RATE: f32 = 8.0;
/// Faster rate for mouse look, so it feels direct.
const LOOK_DECAY_RATE: f32 = 25.0;
/// Distance from the seat below which the camera counts as seated.
const SEATED_DISTANCE: f32 = 0.01;

pub(super) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            PlayerCamera::follow.after(Focused::leave_on_right_click),
        );
    }
}

/// The camera of the seated player.
#[derive(Component, Debug)]
#[require(Camera3d, IsDefaultUiCamera, DebandDither::Disabled)]
pub(crate) struct PlayerCamera {
    /// Pose while sitting at the desk and looking straight ahead.
    seat: Transform,
}

impl PlayerCamera {
    pub(crate) fn seated_at(seat: Transform) -> impl Bundle {
        (Self { seat }, seat)
    }

    pub(crate) fn seat(&self) -> Transform {
        self.seat
    }

    fn follow(
        time: Res<Time>,
        focused: Res<Focused>,
        angles: Res<LookAngles>,
        targets: Query<(&FocusTarget, &GlobalTransform)>,
        camera: Single<(&PlayerCamera, &mut Transform)>,
    ) {
        let (camera, mut transform) = camera.into_inner();
        let focus_view = focused
            .get()
            .and_then(|entity| targets.get(entity).ok())
            .map(|(target, global)| match target.view {
                View::Local(view) => global.mul_transform(view).compute_transform(),
                View::Seated => camera.seat,
            });
        let (goal, decay_rate) = match focus_view {
            Some(view) => (view, DECAY_RATE),
            None => {
                let goal = angles.apply(camera.seat);
                let seated = transform.translation.distance(goal.translation) < SEATED_DISTANCE;
                (goal, if seated { LOOK_DECAY_RATE } else { DECAY_RATE })
            }
        };

        let dt = time.delta_secs();
        transform
            .translation
            .smooth_nudge(&goal.translation, decay_rate, dt);
        transform
            .rotation
            .smooth_nudge(&goal.rotation, decay_rate, dt);
    }
}
