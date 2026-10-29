mod clouds;

use std::error::Error;

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    camera::visibility::NoFrustumCulling,
    light::NotShadowCaster,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use ld60_game_core::clock::{Clock, PLANET_SIZE};
use ld60_game_player::camera::PlayerCamera;

use super::atmosphere::SKY;
use super::weather::{WIND, Weather};

/// Far away, but still inside the camera's far plane.
const RADIUS: f32 = 900.0;
/// The planet's axis, which its rings go around.
const PLANET_AXIS: Vec3 = Vec3::new(0.4, 1.0, -0.1);
/// Towards the sun, which lights the planet from the side.
const SUN: Vec3 = Vec3::new(-1.0, 0.2, 0.6);
/// Light of the planet's lit side.
const PLANET_LIGHT: f32 = 0.55;
const ZENITH: f32 = 0.25;
/// The wind blows this many times stronger at the height of the clouds.
const ALOFT: f32 = 5.0;
/// The cloud noise repeats every this many meters. Matches `SHAPE_SIZE` in
/// `sky.wesl`.
const CLOUD_TILE: f32 = 2400.0;

pub(super) struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "sky.wesl");
        app.add_plugins(MaterialPlugin::<SkyMaterial>::default())
            .add_systems(Startup, SkyMaterial::spawn)
            .add_systems(
                Update,
                (
                    SkyMaterial::follow_camera,
                    SkyMaterial::drift,
                    SkyMaterial::follow_weather.run_if(resource_changed::<Weather>),
                    SkyMaterial::follow_clock.run_if(resource_changed::<Clock>),
                ),
            );
    }
}

/// The night sky: city glow, clouds, stars and a ringed planet.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct SkyMaterial {
    #[uniform(0)]
    params: SkyParams,
    #[texture(1, dimension = "3d")]
    #[sampler(2)]
    clouds: Handle<Image>,
}

/// Matches `Sky` in `sky.wesl`.
#[derive(ShaderType, Debug, Clone)]
struct SkyParams {
    horizon: Vec3,
    zenith: f32,
    planet: Vec3,
    planet_size: f32,
    axis: Vec3,
    planet_light: f32,
    sun: Vec3,
    rain: f32,
    /// How far the wind has blown the clouds, in meters.
    drift: Vec2,
    /// How clear of the buildings the planet is, from 0 to 1.
    planet_clear: f32,
}

/// The dome the sky is drawn on, centered on the camera.
#[derive(Component, Debug)]
struct Dome;

impl SkyMaterial {
    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("sky.wesl"))
            .with_source("embedded")
            .into()
    }

    fn spawn(
        clock: Res<Clock>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<SkyMaterial>>,
        mut images: ResMut<Assets<Image>>,
    ) {
        // Seen from inside.
        let mesh = match Sphere::new(RADIUS)
            .mesh()
            .uv(64, 32)
            .with_inverted_winding()
        {
            Ok(mesh) => mesh,
            Err(err) => {
                error!(error = &err as &dyn Error, "sky dome does not build");
                return;
            }
        };
        commands.spawn((
            Name::new("sky"),
            Dome,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(SkyMaterial {
                params: SkyParams {
                    horizon: SKY.to_linear().to_vec3(),
                    zenith: ZENITH,
                    planet: clock.planet(),
                    planet_size: PLANET_SIZE,
                    axis: PLANET_AXIS.normalize(),
                    planet_light: PLANET_LIGHT,
                    sun: SUN.normalize(),
                    rain: 0.0,
                    drift: Vec2::ZERO,
                    planet_clear: clock.planet_light(),
                },
                clouds: images.add(Self::cloud_noise()),
            })),
            NoFrustumCulling,
            NotShadowCaster,
        ));
    }

    fn follow_camera(
        camera: Single<&GlobalTransform, With<PlayerCamera>>,
        mut dome: Single<&mut Transform, With<Dome>>,
    ) {
        dome.translation = camera.translation();
    }

    /// Blows the clouds along with the wind. Wraps where the noise repeats,
    /// so the drift stays small.
    fn drift(time: Res<Time>, mut materials: ResMut<Assets<SkyMaterial>>) {
        for (_, material) in materials.iter_mut() {
            let drift = material.params.drift + WIND * ALOFT * time.delta_secs();
            material.params.drift = drift.rem_euclid(Vec2::splat(CLOUD_TILE));
        }
    }

    fn follow_weather(weather: Res<Weather>, mut materials: ResMut<Assets<SkyMaterial>>) {
        for (_, material) in materials.iter_mut() {
            material.params.rain = weather.rain;
        }
    }

    /// Moves the planet across the sky with the time.
    fn follow_clock(clock: Res<Clock>, mut materials: ResMut<Assets<SkyMaterial>>) {
        for (_, material) in materials.iter_mut() {
            material.params.planet = clock.planet();
            material.params.planet_clear = clock.planet_light();
        }
    }
}

impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}
