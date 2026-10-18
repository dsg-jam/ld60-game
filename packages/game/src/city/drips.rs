use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use rand::RngExt;

use super::buildings::Skyline;
use super::rng;
use super::weather::Weather;
use crate::GRAVITY;
use crate::geometry::quads::Quads;

const SEED: u64 = 7;
const DROPS_PER_METER: f32 = 4.0;
const PERIOD: (f32, f32) = (0.5, 2.0);
const WIDTH: f32 = 0.008;
const BRIGHTNESS: f32 = 2.5;

pub(super) struct DripsPlugin;

impl Plugin for DripsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "drips.wesl");
        app.add_plugins(MaterialPlugin::<DripMaterial>::default())
            .init_resource::<DripEdges>()
            .add_systems(Startup, DripMaterial::spawn.after(Skyline::spawn))
            .add_systems(
                Update,
                DripMaterial::follow_weather.run_if(resource_changed::<Weather>),
            );
    }
}

/// Edges that water drips from, collected while building the city.
#[derive(Resource, Debug, Default)]
pub(super) struct DripEdges(pub(super) Vec<DripEdge>);

/// A straight edge that drips, lit by `light`.
#[derive(Debug, Clone, Copy)]
pub(super) struct DripEdge {
    pub(super) from: Vec3,
    pub(super) to: Vec3,
    pub(super) light: Color,
}

/// Drops falling from the edges of awnings.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct DripMaterial {
    #[uniform(0)]
    params: DripParams,
}

/// Matches `Drips` in `drips.wesl`.
#[derive(ShaderType, Debug, Clone)]
struct DripParams {
    amount: f32,
    width: f32,
    brightness: f32,
    gravity: f32,
}

impl DripMaterial {
    fn spawn(
        mut commands: Commands,
        edges: Res<DripEdges>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<DripMaterial>>,
    ) {
        commands.spawn((
            Name::new("drips"),
            Mesh3d(meshes.add(Self::drops(&edges.0))),
            MeshMaterial3d(materials.add(DripMaterial {
                params: DripParams {
                    amount: 0.0,
                    width: WIDTH,
                    brightness: BRIGHTNESS,
                    gravity: GRAVITY,
                },
            })),
            Transform::default(),
            Quads::unbounded(),
        ));
    }

    /// One quad per drop along each edge. The vertex shader lets them fall.
    fn drops(edges: &[DripEdge]) -> Mesh {
        let mut rng = rng(SEED);
        let mut quads = Quads::default();
        for edge in edges {
            let count = (edge.from.distance(edge.to) * DROPS_PER_METER).ceil() as usize;
            let light = edge.light.to_linear();
            for _ in 0..count {
                let anchor = edge.from.lerp(edge.to, rng.random::<f32>());
                let timing = [
                    rng.random_range(PERIOD.0..PERIOD.1),
                    rng.random::<f32>(),
                    rng.random::<f32>(),
                ];
                let light = [light.red, light.green, light.blue, 1.0];
                quads.push_with(anchor, timing, light);
            }
        }
        quads.into_mesh()
    }

    fn follow_weather(
        weather: Res<Weather>,
        drips: Query<&MeshMaterial3d<DripMaterial>>,
        mut materials: ResMut<Assets<DripMaterial>>,
    ) {
        for handle in &drips {
            if let Some(mut material) = materials.get_mut(handle) {
                material.params.amount = weather.wetness;
            }
        }
    }

    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("drips.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl Material for DripMaterial {
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
