use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    camera::visibility::NoFrustumCulling,
    light::NotShadowCaster,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use rand::RngExt;

use super::super::rng;
use super::super::weather::Weather;
use super::models::WHEELBASE;
use crate::geometry::quads::Quads;

const SEED: u64 = 6;
/// Droplets per rear wheel. The front wheels throw fewer.
const DROPLETS: usize = 80;
const FRONT_DROPLETS: usize = 24;
/// Distance of the wheels from the middle of the car.
const TRACK: f32 = 0.85;
const LIFETIME: f32 = 0.6;
const SIZE: f32 = 0.4;
const TAIL: f32 = -2.3;

pub(super) struct SprayPlugin;

impl Plugin for SprayPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "spray.wesl");
        app.add_plugins(MaterialPlugin::<SprayMaterial>::default())
            .init_resource::<Spray>()
            .add_systems(
                Update,
                Spray::follow_weather.run_if(resource_changed::<Weather>),
            );
    }
}

/// Water thrown up by the tires of a car on a wet road.
#[derive(Resource, Debug)]
pub(super) struct Spray {
    mesh: Handle<Mesh>,
    material: Handle<SprayMaterial>,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct SprayMaterial {
    #[uniform(0)]
    params: SprayParams,
}

/// Matches `Spray` in `spray.wesl`.
#[derive(ShaderType, Debug, Clone)]
struct SprayParams {
    wetness: f32,
    lifetime: f32,
    size: f32,
    tail: f32,
}

impl FromWorld for Spray {
    fn from_world(world: &mut World) -> Self {
        let mesh = world.resource_mut::<Assets<Mesh>>().add(Self::droplets());
        let material = world
            .resource_mut::<Assets<SprayMaterial>>()
            .add(SprayMaterial {
                params: SprayParams {
                    wetness: 0.0,
                    lifetime: LIFETIME,
                    size: SIZE,
                    tail: TAIL,
                },
            });
        Self { mesh, material }
    }
}

impl Spray {
    /// The spray of one car, as a child of it.
    pub(super) fn bundle(&self) -> impl Bundle {
        (
            Mesh3d(self.mesh.clone()),
            MeshMaterial3d(self.material.clone()),
            Transform::default(),
            // The vertex shader moves the droplets, so the mesh bounds mean
            // nothing.
            NoFrustumCulling,
            NotShadowCaster,
        )
    }

    /// One quad per droplet, all starting at a tire. The vertex shader
    /// throws them.
    fn droplets() -> Mesh {
        let mut rng = rng(SEED);
        let mut quads = Quads::default();
        let wheels = [
            (-WHEELBASE, -1.0, DROPLETS),
            (-WHEELBASE, 1.0, DROPLETS),
            (WHEELBASE, -1.0, FRONT_DROPLETS),
            (WHEELBASE, 1.0, FRONT_DROPLETS),
        ];
        for (x, side, count) in wheels {
            for _ in 0..count {
                let random = [
                    rng.random::<f32>(),
                    rng.random::<f32>(),
                    rng.random::<f32>(),
                    side,
                ];
                quads.push_with(Vec3::new(x - 0.3, 0.05, side * TRACK), [0.0; 3], random);
            }
        }
        quads.into_mesh()
    }

    fn follow_weather(
        weather: Res<Weather>,
        spray: Res<Spray>,
        mut materials: ResMut<Assets<SprayMaterial>>,
    ) {
        if let Some(mut material) = materials.get_mut(&spray.material) {
            material.params.wetness = weather.wetness;
        }
    }
}

impl SprayMaterial {
    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("spray.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl Material for SprayMaterial {
    fn vertex_shader() -> ShaderRef {
        Self::shader()
    }

    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}
