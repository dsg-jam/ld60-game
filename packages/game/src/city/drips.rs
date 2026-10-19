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
/// Seconds a drop takes at least to form after the last one fell.
const FORMING: f32 = 0.2;
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

/// Spots along the [`DripEdges`] where drops form and fall.
#[derive(Resource, Debug)]
pub(super) struct Drips(Vec<Drip>);

/// A spot where a drop forms, falls at a random time in each round of
/// `period` seconds, or now and then skips a round.
#[derive(Debug, Clone, Copy)]
pub(super) struct Drip {
    pub(super) anchor: Vec3,
    period: f32,
    /// A whole number that picks the random numbers of each round.
    seed: f32,
    /// Where in its round the drip is at time 0, from 0 to 1.
    phase: f32,
    /// The light the drop falls through.
    light: LinearRgba,
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
        let drips = Drips::along(&edges.0);
        commands.spawn((
            Name::new("drips"),
            Mesh3d(meshes.add(drips.mesh())),
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
        commands.insert_resource(drips);
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

impl Drips {
    fn along(edges: &[DripEdge]) -> Self {
        let mut rng = rng(SEED);
        let mut drips = Vec::new();
        for edge in edges {
            let count = (edge.from.distance(edge.to) * DROPS_PER_METER).ceil() as usize;
            for _ in 0..count {
                let anchor = edge.from.lerp(edge.to, rng.random::<f32>());
                let mut drip = Drip {
                    anchor,
                    period: rng.random_range(PERIOD.0..PERIOD.1),
                    seed: rng.random_range(0..1 << 16) as f32,
                    phase: rng.random::<f32>(),
                    light: edge.light.to_linear(),
                };
                drip.period = drip.period.max(drip.fall() + FORMING);
                drips.push(drip);
            }
        }
        Self(drips)
    }

    /// One quad per drop. The vertex shader lets them fall.
    fn mesh(&self) -> Mesh {
        let mut quads = Quads::default();
        for drip in &self.0 {
            let light = drip.light;
            quads.push_with(
                drip.anchor,
                [drip.period, drip.seed, drip.phase],
                [light.red, light.green, light.blue, 1.0],
            );
        }
        quads.into_mesh()
    }
}

impl Drip {
    /// Seconds a drop takes to fall to the ground.
    fn fall(&self) -> f32 {
        (2.0 * self.anchor.y / GRAVITY).sqrt()
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
