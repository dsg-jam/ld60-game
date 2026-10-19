mod additions;
mod atmosphere;
mod buildings;
mod cables;
mod drips;
mod market;
mod rain;
mod signs;
mod sky;
mod soundscape;
mod street;
mod traffic;
mod weather;
mod wet;

use std::f32::consts::PI;

use bevy::{
    light::cluster::GlobalClusterSettings,
    prelude::*,
    shape::{Aabb3d, RayCast3d},
};
use rand::rngs::SmallRng;
use rand::{Rng, RngExt, SeedableRng};

use crate::geometry::bar::Bar;
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
/// Height of the player's eyes above the street.
const EYE_HEIGHT: f32 = 1.5;
/// The colors of the city's neon: signs, crowns on the towers and lights
/// under the cars.
const NEON: [Color; 6] = [
    Color::srgb_u8(0xff, 0x2a, 0x6d),
    Color::srgb_u8(0x05, 0xd9, 0xe8),
    Color::srgb_u8(0xb1, 0x4a, 0xed),
    Color::srgb_u8(0x7d, 0xff, 0x5a),
    Color::srgb_u8(0xff, 0xb3, 0x2e),
    Color::srgb_u8(0xff, 0x4f, 0x2e),
];

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
            soundscape::SoundscapePlugin,
            street::StreetPlugin,
            traffic::TrafficPlugin,
            weather::WeatherPlugin,
            wet::WetPlugin,
        ))
        .init_resource::<Kit>()
        .add_systems(Startup, make_room_for_lights);
    }
}

/// A straight piece of a path, like a cable or a flight route.
#[derive(Clone, Copy, Debug)]
struct Segment {
    from: Vec3,
    to: Vec3,
}

impl Segment {
    /// Whether the segment passes `bounds` by.
    fn misses(&self, bounds: &Aabb3d) -> bool {
        let Ok(direction) = Dir3::new(self.to - self.from) else {
            return true;
        };
        RayCast3d::new(self.from, direction, self.from.distance(self.to))
            .aabb_intersection_at(bounds)
            .is_none()
    }
}

/// Picks one item at random, like `rand`'s `choose`, from a list that is
/// never empty.
trait Pick<T> {
    fn pick(&self, rng: &mut impl Rng) -> &T;
}

impl<T> Pick<T> for [T] {
    fn pick(&self, rng: &mut impl Rng) -> &T {
        &self[rng.random_range(0..self.len())]
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

    fn bar<M: Material>(&self, bar: &Bar, material: Handle<M>) -> Part<M> {
        (
            Mesh3d(self.cube.clone()),
            MeshMaterial3d(material),
            bar.unit_transform(),
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

/// No turn for a positive `side`, half a turn about Y for a negative one,
/// for things that face either way across or along the street.
fn turned_to(side: f32) -> Quat {
    if side > 0.0 {
        Quat::IDENTITY
    } else {
        Quat::from_rotation_y(PI)
    }
}
