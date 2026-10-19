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

use super::focus::{Aside, FocusTarget, Focused, View};
use super::look::LookAngles;
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::menu::{AppState, Settings};

/// How quickly the camera moves to a new view.
const DECAY_RATE: f32 = 8.0;
/// Faster rate for mouse look, so it feels direct.
const LOOK_DECAY_RATE: f32 = 25.0;
/// Distance from the seat below which the camera counts as seated.
const SEATED_DISTANCE: f32 = 0.01;
/// Slower rate for long moves, like from the street to the seat.
const TRAVEL_DECAY_RATE: f32 = 2.0;
/// Moves longer than this, in meters, use [`TRAVEL_DECAY_RATE`].
const TRAVEL_DISTANCE: f32 = 1.5;
/// On a long move the camera keeps facing ahead until it is this close, about
/// over the counter, and only then turns.
const TURN_DISTANCE: f32 = 1.0;
/// A long move ends once the camera is this close to its goal, in meters.
const ARRIVED: f32 = 0.02;

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
                        .run_if(not(in_state(AppState::Title))),
                    PlayerCamera::match_quality.run_if(
                        resource_changed::<Settings>
                            .or_else(any_match_filter::<Added<PlayerCamera>>),
                    ),
                ),
            );
        #[cfg(feature = "debug_port")]
        app.init_resource::<CameraOverride>()
            .add_systems(Update, CameraOverride::apply.after(PlayerCamera::follow))
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
pub(crate) struct PlayerCamera {
    /// Pose while sitting at the counter, before looking around.
    seat: Transform,
    /// On a long move, like from the street to the seat.
    traveling: bool,
}

impl PlayerCamera {
    pub(crate) fn seated_at(seat: Transform) -> impl Bundle {
        (
            Self {
                seat,
                traveling: false,
            },
            seat,
        )
    }

    pub(crate) fn seat(&self) -> Transform {
        self.seat
    }

    /// Whether the camera at `at` has come to rest on the seat.
    pub(crate) fn seated(&self, at: &Transform) -> bool {
        !self.traveling && at.translation.distance(self.seat.translation) < SEATED_DISTANCE
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
                    ..default()
                }),
                None => camera.remove::<ScreenSpaceAmbientOcclusion>(),
            };
        }
    }

    fn follow(
        time: Res<Time>,
        focused: Res<Focused>,
        angles: Res<LookAngles>,
        targets: Query<(&FocusTarget, Option<&Aside>, &GlobalTransform)>,
        camera: Single<(&mut PlayerCamera, &mut Transform)>,
    ) {
        let (mut camera, mut transform) = camera.into_inner();
        let focus_view = focused
            .get()
            .and_then(|entity| targets.get(entity).ok())
            .map(|(target, aside, global)| {
                let view = match aside.filter(|aside| aside.open) {
                    Some(aside) => View::Local(aside.view),
                    None => target.view,
                };
                match view {
                    View::Local(view) => global.mul_transform(view).compute_transform(),
                    View::World(view) => view,
                }
            });
        let (goal, decay_rate) = match focus_view {
            Some(view) => (view, DECAY_RATE),
            None => {
                let goal = angles.apply(camera.seat);
                let seated = transform.translation.distance(goal.translation) < SEATED_DISTANCE;
                (goal, if seated { LOOK_DECAY_RATE } else { DECAY_RATE })
            }
        };

        let distance = transform.translation.distance(goal.translation);
        if distance > TRAVEL_DISTANCE {
            camera.traveling = true;
        } else if distance < ARRIVED {
            camera.traveling = false;
        }
        let (decay_rate, turning) = if camera.traveling {
            (TRAVEL_DECAY_RATE, distance < TURN_DISTANCE)
        } else {
            (decay_rate, true)
        };
        let dt = time.delta_secs();
        transform
            .translation
            .smooth_nudge(&goal.translation, decay_rate, dt);
        if turning {
            transform
                .rotation
                .smooth_nudge(&goal.rotation, decay_rate, dt);
        }
    }
}

/// A camera pose set over the debug port, kept until it is cleared.
#[cfg(feature = "debug_port")]
#[derive(Resource, Default, Debug)]
struct CameraOverride(Option<Transform>);

#[cfg(feature = "debug_port")]
impl CameraOverride {
    fn apply(pose: Res<CameraOverride>, mut camera: Single<&mut Transform, With<PlayerCamera>>) {
        if let Some(pose) = pose.0 {
            **camera = pose;
        }
    }

    /// `ld60.camera { "eye": [x, y, z], "at": [x, y, z] }` holds the camera
    /// at `eye`, looking at `at`. Without parameters the camera is released.
    fn debug_camera(
        In(params): In<Option<serde_json::Value>>,
        mut pose: ResMut<CameraOverride>,
    ) -> bevy::remote::BrpResult {
        use bevy::remote::{BrpError, error_codes};

        let Some(params) = params.filter(|params| !params.is_null()) else {
            pose.0 = None;
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
        pose.0 = Some(Transform::from_translation(point("eye")?).looking_at(point("at")?, Vec3::Y));
        Ok(serde_json::Value::Null)
    }
}
