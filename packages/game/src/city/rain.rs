use bevy::{
    asset::RenderAssetUsages,
    asset::{AssetPath, embedded_asset, embedded_path},
    camera::visibility::NoFrustumCulling,
    light::NotShadowCaster,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use rand::RngExt;

use super::weather::Weather;
use super::{FACADES, rng};
use crate::player::camera::PlayerCamera;
use crate::workshop::shop;

const SEED: u64 = 5;
const STREAKS: usize = 6000;
/// The rain falls over the street in front of the shop.
const HALF_WIDTH: f32 = 20.0;
const HEIGHT: f32 = 14.0;
const WIDTH: f32 = 0.006;
const LENGTH: f32 = 0.4;
/// Speed and direction in meters per second, tilted by the wind.
const FALL: Vec3 = Vec3::new(1.2, -9.0, 0.6);
const AMBIENT: f32 = 0.03;
/// Matches `LIGHTS` in `rain.wesl`.
const LIGHTS: usize = 8;
/// Converts the lumens of spot and point lights to rain brightness.
const SPOT_SCALE: f32 = 5e-6;
const POINT_SCALE: f32 = 2e-5;

pub(super) struct RainPlugin;

impl Plugin for RainPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "rain.wesl");
        app.add_plugins(MaterialPlugin::<RainMaterial>::default())
            .add_systems(Startup, RainMaterial::spawn)
            .add_systems(Update, RainMaterial::light_up);
    }
}

/// Streaks of falling rain, lit by the lights around them.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct RainMaterial {
    #[uniform(0)]
    params: RainParams,
}

/// Matches `Rain` in `rain.wesl`.
#[derive(ShaderType, Debug, Clone)]
struct RainParams {
    origin: Vec3,
    width: f32,
    size: Vec3,
    length: f32,
    fall: Vec3,
    ambient: f32,
    /// Share of the streaks that fall, from 0 to 1.
    amount: f32,
    lights: [Vec4; LIGHTS],
    colors: [Vec4; LIGHTS],
}

impl RainMaterial {
    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("rain.wesl"))
            .with_source("embedded")
            .into()
    }

    fn spawn(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<RainMaterial>>,
    ) {
        let origin = Vec3::new(-HALF_WIDTH, 0.0, FACADES);
        let size = Vec3::new(2.0 * HALF_WIDTH, HEIGHT, shop::FRONT - FACADES);
        commands.spawn((
            Name::new("rain"),
            Mesh3d(meshes.add(Self::streaks())),
            MeshMaterial3d(materials.add(RainMaterial {
                params: RainParams {
                    origin,
                    width: WIDTH,
                    size,
                    length: LENGTH,
                    fall: FALL,
                    ambient: AMBIENT,
                    amount: 0.0,
                    lights: [Vec4::ZERO; LIGHTS],
                    colors: [Vec4::ZERO; LIGHTS],
                },
            })),
            Transform::default(),
            // The vertex shader moves the streaks, so the mesh bounds mean nothing.
            NoFrustumCulling,
            NotShadowCaster,
        ));
    }

    /// One quad per streak. All corners share a random point, the vertex
    /// shader turns it into the falling streak.
    fn streaks() -> Mesh {
        let mut rng = rng(SEED);
        let mut seeds = Vec::with_capacity(STREAKS * 4);
        let mut uvs = Vec::with_capacity(STREAKS * 4);
        let mut indices = Vec::with_capacity(STREAKS * 6);
        for streak in 0..STREAKS as u32 {
            let seed = [
                rng.random::<f32>(),
                rng.random::<f32>(),
                rng.random::<f32>(),
            ];
            seeds.extend([seed; 4]);
            uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
            let base = streak * 4;
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, seeds)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
    }

    /// Picks the lights that matter most near the player.
    fn light_up(
        camera: Single<&GlobalTransform, With<PlayerCamera>>,
        spots: Query<(&SpotLight, &GlobalTransform)>,
        points: Query<(&PointLight, &GlobalTransform)>,
        weather: Res<Weather>,
        rains: Query<&MeshMaterial3d<RainMaterial>>,
        mut materials: ResMut<Assets<RainMaterial>>,
    ) {
        let eye = camera.translation();
        let mut lights: Vec<(Vec3, f32, Color)> =
            spots
                .iter()
                .map(|(light, at)| (at.translation(), light.intensity * SPOT_SCALE, light.color))
                .chain(points.iter().map(|(light, at)| {
                    (at.translation(), light.intensity * POINT_SCALE, light.color)
                }))
                .filter(|(at, _, _)| at.z < shop::FRONT)
                .collect();
        let weight =
            |(at, strength, _): &(Vec3, f32, Color)| strength / (1.0 + at.distance_squared(eye));
        lights.sort_by(|a, b| weight(b).total_cmp(&weight(a)));
        for handle in &rains {
            let Some(mut material) = materials.get_mut(handle) else {
                continue;
            };
            let params = &mut material.params;
            params.amount = weather.rain;
            params.lights = [Vec4::ZERO; LIGHTS];
            params.colors = [Vec4::ZERO; LIGHTS];
            for (index, (at, strength, color)) in lights.iter().take(LIGHTS).enumerate() {
                params.lights[index] = at.extend(*strength);
                params.colors[index] = color.to_linear().to_vec4();
            }
        }
    }
}

impl Material for RainMaterial {
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
