mod additions;
mod atmosphere;
mod buildings;
mod cables;
mod drips;
mod market;
mod rain;
mod signs;
mod sky;
mod street;
mod traffic;
mod weather;
mod wet;

use bevy::{light::cluster::GlobalClusterSettings, prelude::*};
use rand::SeedableRng;
use rand::rngs::SmallRng;

use crate::geometry::block::Block;

/// Curb on the shop side of the road.
const NEAR_CURB: f32 = -4.0;
/// Curb on the far side of the road.
const FAR_CURB: f32 = -13.0;
/// Front of the buildings across the road.
const FACADES: f32 = -16.0;
/// How far the road drops below the sidewalks.
const CURB_HEIGHT: f32 = 0.15;
/// Room for light clustering, see [`make_room_for_lights`].
const Z_SLICES: usize = 4096;
/// The street ends this far to each side of the shop.
const HALF_LENGTH: f32 = 90.0;

pub(crate) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            atmosphere::AtmospherePlugin,
            buildings::BuildingsPlugin,
            cables::CablesPlugin,
            drips::DripsPlugin,
            rain::RainPlugin,
            signs::SignsPlugin,
            sky::SkyPlugin,
            street::StreetPlugin,
            traffic::TrafficPlugin,
            weather::WeatherPlugin,
            wet::WetPlugin,
        ))
        .init_resource::<Kit>()
        .add_systems(Startup, make_room_for_lights);
    }
}

/// A shared mesh placed and scaled to fill a [`Block`].
type Part<M> = (Mesh3d, MeshMaterial3d<M>, Transform);

/// Shared meshes for the many boxes of the city.
#[derive(Resource, Debug)]
struct Kit {
    cube: Handle<Mesh>,
}

impl FromWorld for Kit {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        Self {
            cube: meshes.add(Cuboid::from_length(1.0)),
        }
    }
}

impl Kit {
    fn cuboid<M: Material>(&self, block: &Block, material: Handle<M>) -> Part<M> {
        (
            Mesh3d(self.cube.clone()),
            MeshMaterial3d(material),
            block.transform().with_scale(block.size),
        )
    }

    /// A bar from `a` to `b`, `thickness` wide and deep.
    fn bar<M: Material>(&self, a: Vec3, b: Vec3, thickness: f32, material: Handle<M>) -> Part<M> {
        let along = b - a;
        let up = if along.normalize_or_zero().y.abs() > 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        (
            Mesh3d(self.cube.clone()),
            MeshMaterial3d(material),
            Transform::from_translation((a + b) / 2.0)
                .looking_to(along, up)
                .with_scale(Vec3::new(thickness, thickness, along.length())),
        )
    }
}

/// The street has more lights than Bevy expects by default. Without more room
/// for them, the lighting breaks for a few frames when it grows.
fn make_room_for_lights(mut settings: ResMut<GlobalClusterSettings>) {
    if let Some(gpu) = &mut settings.gpu_clustering {
        gpu.initial_z_slice_list_capacity = gpu.initial_z_slice_list_capacity.max(Z_SLICES);
    }
}

/// A matte surface.
fn matte(color: Color, roughness: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        perceptual_roughness: roughness,
        ..default()
    }
}

/// A light source surface, like a neon tube or a lamp.
fn glow(color: Color, strength: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::BLACK,
        emissive: color.to_linear() * strength,
        ..default()
    }
}

/// The same city every time.
fn rng(seed: u64) -> SmallRng {
    SmallRng::seed_from_u64(seed)
}
