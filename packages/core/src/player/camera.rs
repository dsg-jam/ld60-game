use bevy::{
    anti_alias::smaa::Smaa,
    camera::Hdr,
    core_pipeline::{
        prepass::{DeferredPrepass, DepthPrepass, NormalPrepass},
        tonemapping::{DebandDither, Tonemapping},
    },
    ecs::schedule::common_conditions::any_match_filter,
    pbr::{DefaultOpaqueRendererMethod, ScreenSpaceAmbientOcclusion},
    post_process::bloom::Bloom,
    prelude::*,
};

use super::focus::{Aside, FocusTarget, Focused};
use super::free_look::FreeLook;
use super::lens::FOV;
use super::look::LookAngles;
use super::zoom::Zoom;
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::settings::Settings;
use crate::state::AppState;

/// How quickly the camera moves to a new view.
const DECAY_RATE: f32 = 8.0;
/// Faster rate for mouse look, so it feels direct.
const LOOK_DECAY_RATE: f32 = 25.0;
/// Distance from the seat below which the camera counts as seated.
const SEATED_DISTANCE: f32 = 0.01;
/// Moves longer than this, in meters, are a [`Travel`].
const TRAVEL_DISTANCE: f32 = 1.5;
/// Seconds for a [`Travel`] to reach its goal.
const TRAVEL_TIME: f32 = 1.6;
/// On a [`Travel`] the camera keeps facing ahead until it is this close,
/// about over the counter, and only then turns.
const TURN_DISTANCE: f32 = 1.0;
/// Seconds for the turn at the end of a [`Travel`].
const TURN_TIME: f32 = 0.6;
/// How far through the turn of a [`Travel`] the player can take over.
const TAKE_OVER_TURN: f32 = 0.5;
/// How far ambient occlusion reaches, in meters. Things on the desk are a
/// few centimeters big, so Bevy's default of most of a meter darkens a sheet
/// over the whole stack it lies on.
const OCCLUSION_RADIUS: f32 = 0.15;
/// How thick things are assumed to be, in meters, so light passes behind a
/// sheet of paper.
const OCCLUSION_THICKNESS: f32 = 0.03;

pub(super) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        // Deferred shading, so many lights stay cheap and screen space
        // effects get the normals and roughness they need.
        app.insert_resource(DefaultOpaqueRendererMethod::deferred())
            .add_systems(
                Update,
                (
                    PlayerCamera::follow
                        .after(Focused::leave_on_input)
                        .after(LookAngles::update)
                        .after(FreeLook::update)
                        .run_if(not(
                            in_state(AppState::Loading).or_eager(in_state(AppState::Title))
                        )),
                    PlayerCamera::match_quality.run_if(
                        resource_changed::<Settings>
                            .or_else(any_match_filter::<Added<PlayerCamera>>),
                    ),
                ),
            )
            .add_systems(OnEnter(AppState::Title), PlayerCamera::stop_travel)
            .add_systems(
                PostUpdate,
                PlayerCamera::match_bloom.before(bevy::camera::CameraUpdateSystems),
            );
        #[cfg(feature = "debug_port")]
        app.init_resource::<CameraOverride>()
            .add_systems(
                PostUpdate,
                CameraOverride::apply
                    .before(bevy::transform::TransformSystems::Propagate)
                    .before(bevy::camera::CameraUpdateSystems),
            )
            .add_debug_method("ld60.camera", CameraOverride::debug_camera);
    }
}

/// The camera of the seated player.
#[derive(Component, Debug)]
#[require(
    Camera3d,
    IsDefaultUiCamera,
    DebandDither::Disabled,
    Hdr,
    Bloom = Bloom::NATURAL,
    Tonemapping::TonyMcMapface,
    DeferredPrepass,
    // Without these the deferred lighting keeps the size of the first
    // window after a resize, unless an effect that needs them came first.
    DepthPrepass,
    NormalPrepass,
    // Deferred shading doesn't work with multisampling.
    Msaa::Off,
    Smaa,
)]
pub struct PlayerCamera {
    /// Pose while sitting at the counter, before looking around.
    seat: Transform,
    travel: Option<Travel>,
}

impl PlayerCamera {
    /// Bloom blurs over a share of the screen. A narrower view would blur
    /// over a smaller part of the scene, so the glow of many small far
    /// lights, which looks like haze, would thin out as the player zooms
    /// in. Stretching it by the zoom keeps it the same.
    fn match_bloom(cameras: Query<(&Projection, &mut Bloom), With<PlayerCamera>>) {
        for (projection, mut bloom) in cameras {
            let Projection::Perspective(perspective) = projection else {
                continue;
            };
            let zoom = (FOV / 2.0).tan() / (perspective.fov / 2.0).tan();
            let scale = Vec2::splat(zoom.max(1.0));
            if bloom.scale != scale {
                bloom.scale = scale;
            }
        }
    }

    pub fn seated_at(seat: Transform) -> impl Bundle {
        (Self { seat, travel: None }, seat)
    }

    pub fn seat(&self) -> Transform {
        self.seat
    }

    /// Whether the camera at `at` has come to rest on the seat.
    pub fn seated(&self, at: &Transform) -> bool {
        self.travel.is_none() && at.translation.distance(self.seat.translation) < SEATED_DISTANCE
    }

    /// Whether the player may look around, which on a [`Travel`] starts
    /// during the turn.
    pub(super) fn steerable(&self) -> bool {
        self.travel
            .as_ref()
            .is_none_or(|travel| travel.turned() >= TAKE_OVER_TURN)
    }

    /// Lets the player turn the camera for the rest of a [`Travel`]. Returns
    /// whether the camera was still turning on its own.
    pub(super) fn take_over(&mut self) -> bool {
        let Some(travel) = self.travel.as_mut().filter(|travel| !travel.manual) else {
            return false;
        };
        travel.manual = true;
        true
    }

    fn stop_travel(mut cameras: Query<&mut PlayerCamera>) {
        for mut camera in &mut cameras {
            camera.travel = None;
        }
    }

    /// Ambient occlusion grounds small things on the surfaces they rest on.
    fn match_quality(
        settings: Res<Settings>,
        cameras: Query<Entity, With<PlayerCamera>>,
        mut commands: Commands,
    ) {
        for camera in &cameras {
            let mut camera = commands.entity(camera);
            match settings.quality().ambient_occlusion() {
                Some(quality_level) => camera.insert(ScreenSpaceAmbientOcclusion {
                    quality_level,
                    radius: OCCLUSION_RADIUS,
                    constant_object_thickness: OCCLUSION_THICKNESS,
                }),
                None => camera.remove::<ScreenSpaceAmbientOcclusion>(),
            };
        }
    }

    fn follow(
        time: Res<Time>,
        focused: Res<Focused>,
        angles: Res<LookAngles>,
        free_look: Res<FreeLook>,
        targets: Query<(
            &FocusTarget,
            Option<&Aside>,
            Option<&Zoom>,
            &GlobalTransform,
        )>,
        camera: Single<(&mut PlayerCamera, &mut Transform)>,
    ) {
        let (mut camera, mut transform) = camera.into_inner();
        let focus_view = focused
            .get()
            .and_then(|entity| targets.get(entity).ok())
            .map(|(target, aside, zoom, global)| {
                let view = match aside.filter(|aside| aside.open) {
                    Some(aside) => global.mul_transform(aside.view).compute_transform(),
                    None => {
                        let view = target.view.resolve(global);
                        zoom.map_or(view, |zoom| zoom.apply(view, global))
                    }
                };
                free_look.turn(view)
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
        if camera
            .travel
            .as_ref()
            .is_some_and(|travel| travel.to.distance(goal.translation) > SEATED_DISTANCE)
        {
            camera.travel = None;
        }
        if camera.travel.is_none()
            && transform.translation.distance(goal.translation) > TRAVEL_DISTANCE
        {
            camera.travel = Some(Travel::new(*transform, goal.translation));
        }
        if let Some(travel) = &mut camera.travel {
            if travel.step(&mut transform, goal, dt) {
                camera.travel = None;
            }
            return;
        }
        transform
            .translation
            .smooth_nudge(&goal.translation, decay_rate, dt);
        let turn_rate = if free_look.looking() {
            LOOK_DECAY_RATE
        } else {
            decay_rate
        };
        transform
            .rotation
            .smooth_nudge(&goal.rotation, turn_rate, dt);
    }
}

/// A long move, like from the street to the seat, that eases in and out
/// over [`TRAVEL_TIME`].
#[derive(Debug)]
struct Travel {
    from: Transform,
    to: Vec3,
    elapsed: f32,
    /// When the camera started to turn towards the goal.
    turn_start: Option<f32>,
    /// Whether the player turns the camera, so only the move is left.
    manual: bool,
}

impl Travel {
    fn new(from: Transform, to: Vec3) -> Self {
        Self {
            from,
            to,
            elapsed: 0.0,
            turn_start: None,
            manual: false,
        }
    }

    /// Moves `transform` on towards `goal`. Returns whether it arrived.
    fn step(&mut self, transform: &mut Transform, goal: Transform, dt: f32) -> bool {
        self.elapsed += dt;
        let moved = EaseFunction::SmoothStep.sample_clamped(self.elapsed / TRAVEL_TIME);
        transform.translation = self.from.translation.lerp(goal.translation, moved);
        if self.turn_start.is_none()
            && transform.translation.distance(goal.translation) < TURN_DISTANCE
        {
            self.turn_start = Some(self.elapsed);
        }
        let turned = self.turned();
        if self.manual {
            transform
                .rotation
                .smooth_nudge(&goal.rotation, LOOK_DECAY_RATE, dt);
        } else {
            transform.rotation = self.from.rotation.slerp(goal.rotation, turned);
        }
        moved >= 1.0 && (self.manual || turned >= 1.0)
    }

    /// How far the camera turned towards the goal, from 0 to 1.
    fn turned(&self) -> f32 {
        self.turn_start.map_or(0.0, |start| {
            EaseFunction::SmoothStep.sample_clamped((self.elapsed - start) / TURN_TIME)
        })
    }
}

/// A camera pose set over the debug port, kept until it is cleared. It
/// sees things right in front of it and can narrow the view for close
/// looks.
#[cfg(feature = "debug_port")]
#[derive(Resource, Default, Debug)]
struct CameraOverride {
    pose: Option<Transform>,
    /// In radians.
    fov: Option<f32>,
    /// Whether the projection still has to go back to normal.
    held: bool,
}

#[cfg(feature = "debug_port")]
impl CameraOverride {
    /// Near enough for a screw head a few millimeters away.
    const NEAR: f32 = 0.001;

    fn apply(
        mut held: ResMut<CameraOverride>,
        camera: Single<(&mut Transform, &mut Projection), With<PlayerCamera>>,
    ) {
        let (mut transform, mut projection) = camera.into_inner();
        let Projection::Perspective(perspective) = &mut *projection else {
            return;
        };
        match held.pose {
            Some(pose) => {
                *transform = pose;
                perspective.near = Self::NEAR;
                if let Some(fov) = held.fov {
                    perspective.fov = fov;
                }
                held.held = true;
            }
            None if held.held => {
                perspective.near = PerspectiveProjection::default().near;
                held.held = false;
            }
            None => {}
        }
    }

    /// `ld60.camera { "eye": [x, y, z], "at": [x, y, z], "fov": degrees }`
    /// holds the camera at `eye`, looking at `at`, with a near plane of a
    /// millimeter. `fov` is optional. Without parameters the camera is
    /// released.
    fn debug_camera(
        In(params): In<Option<serde_json::Value>>,
        mut pose: ResMut<CameraOverride>,
    ) -> bevy::remote::BrpResult {
        use bevy::remote::{BrpError, error_codes};

        let Some(params) = params.filter(|params| !params.is_null()) else {
            pose.pose = None;
            return Ok(serde_json::Value::Null);
        };
        let point = |name: &str| {
            params
                .get(name)
                .and_then(serde_json::Value::as_array)
                .and_then(|xyz| {
                    let axis = |i: usize| xyz.get(i)?.as_f64().map(|v| v as f32);
                    Some(Vec3::new(axis(0)?, axis(1)?, axis(2)?))
                })
                .ok_or_else(|| BrpError {
                    code: error_codes::INVALID_PARAMS,
                    message: format!("expected `{name}` as [x, y, z]"),
                    data: None,
                })
        };
        let (eye, at) = (point("eye")?, point("at")?);
        // Straight down needs another up.
        let up = if (at - eye).normalize_or_zero().y.abs() > 0.99 {
            Vec3::NEG_Z
        } else {
            Vec3::Y
        };
        pose.pose = Some(Transform::from_translation(eye).looking_at(at, up));
        pose.fov = params
            .get("fov")
            .and_then(serde_json::Value::as_f64)
            .map(|degrees| (degrees as f32).to_radians());
        Ok(serde_json::Value::Null)
    }
}

/// The pose turned to `rotation` that shows all of `points` as large as
/// `perspective` allows, with `margin` of the view's height and width left
/// free on each side.
pub fn framing(
    rotation: Quat,
    points: impl IntoIterator<Item = Vec3>,
    perspective: &PerspectiveProjection,
    margin: f32,
) -> Transform {
    let slope_y = (perspective.fov / 2.0).tan() * (1.0 - 2.0 * margin);
    let slope_x = slope_y * perspective.aspect_ratio;
    // How far the camera has to move along its own axes so each point lies
    // inside the top, bottom, right and left planes of the frustum.
    let mut reach = [f32::NEG_INFINITY; 4];
    for point in points {
        let point = rotation.inverse() * point;
        let needed = [
            point.y + slope_y * point.z,
            -point.y + slope_y * point.z,
            point.x + slope_x * point.z,
            -point.x + slope_x * point.z,
        ];
        for (reach, needed) in reach.iter_mut().zip(needed) {
            *reach = reach.max(needed);
        }
    }
    let [top, bottom, right, left] = reach;
    let eye = Vec3::new(
        (right - left) / 2.0,
        (top - bottom) / 2.0,
        ((top + bottom) / (2.0 * slope_y)).max((right + left) / (2.0 * slope_x)),
    );
    Transform::from_translation(rotation * eye).with_rotation(rotation)
}

#[cfg(test)]
mod tests {
    use bevy::camera::CameraProjection;

    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn street() -> Transform {
        Transform::from_xyz(2.0, 1.5, -5.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y)
    }

    fn seat() -> Transform {
        Transform::from_xyz(0.0, 1.55, 0.7).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y)
    }

    #[test]
    fn a_travel_faces_ahead_then_settles_on_time() {
        let mut transform = street();
        let mut travel = Travel::new(transform, seat().translation);
        let mut seconds = 0.0;
        while !travel.step(&mut transform, seat(), DT) {
            seconds += DT;
            if transform.translation.distance(seat().translation) > TURN_DISTANCE {
                assert_eq!(transform.rotation, street().rotation);
            }
            assert!(seconds < TRAVEL_TIME + TURN_TIME);
        }
        assert_eq!(transform, seat());
    }

    #[test]
    fn framing_fits_the_points_tightly() {
        let points = [
            Vec3::new(-0.15, 0.08, 0.18),
            Vec3::new(0.15, 0.3, 0.18),
            Vec3::new(0.0, 0.0, 0.37),
        ];
        let rotation = Quat::from_rotation_x(-0.3) * Quat::from_rotation_y(0.2);
        for aspect_ratio in [0.8, 4.0 / 3.0, 16.0 / 9.0, 3.0] {
            let perspective = PerspectiveProjection {
                aspect_ratio,
                ..default()
            };
            let pose = framing(rotation, points, &perspective, 0.05);
            let clip = perspective.get_clip_from_view() * pose.to_matrix().inverse();
            let ndc = points.map(|point| clip.project_point3(point).truncate());
            let reach = ndc
                .iter()
                .fold(Vec2::ZERO, |reach, ndc| reach.max(ndc.abs()));
            assert!(reach.max_element() < 0.9 + 1e-4, "{aspect_ratio} {ndc:?}");
            assert!(reach.max_element() > 0.9 - 1e-4, "{aspect_ratio} {ndc:?}");
        }
    }

    #[test]
    fn taking_over_keeps_the_view() {
        let mut camera = PlayerCamera {
            seat: seat(),
            travel: Some(Travel::new(street(), seat().translation)),
        };
        let mut transform = street();
        while !camera.steerable() {
            camera
                .travel
                .as_mut()
                .unwrap()
                .step(&mut transform, seat(), DT);
        }
        assert!(camera.take_over());
        assert!(!camera.take_over());
        let before = transform.rotation;
        camera.travel.as_mut().unwrap().step(
            &mut transform,
            Transform {
                rotation: before,
                ..seat()
            },
            DT,
        );
        assert!(transform.rotation.angle_between(before) < 1e-3);
    }
}
