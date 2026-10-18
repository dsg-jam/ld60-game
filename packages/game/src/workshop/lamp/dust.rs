use bevy::{
    light::{FogVolume, NotShadowCaster},
    prelude::*,
};
use rand::{RngExt, SeedableRng, rngs::SmallRng};

use super::Lamp;
use crate::geometry::block::Block;
use crate::workshop::shop;

const SEED: u64 = 11;
const MOTES: usize = 160;
const MOTE_RADIUS: f32 = 0.0009;
/// Where the motes float around the lamp, in its own space.
const CLOUD: Block = Block {
    center: Vec3::new(0.2, 0.3, 0.0),
    size: Vec3::new(0.8, 0.6, 0.8),
};
/// Meters per second.
const DRIFT: f32 = 0.012;
/// Warm air rises over the desk.
const RISE: f32 = 0.004;
const MOTE: Color = Color::srgb(0.9, 0.88, 0.85);

/// Dusty air in the shop, which the lamp's light shows up in.
const HAZE: Color = Color::srgb_u8(0xd8, 0xcc, 0xb8);
const HAZE_DENSITY: f32 = 0.012;
const HAZE_GLOW: f32 = 1.0;

pub(super) struct DustPlugin;

impl Plugin for DustPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Mote::spawn_haze)
            .add_systems(Update, Mote::drift)
            .add_observer(Mote::spawn_cloud);
    }
}

/// A speck of dust floating in the air. It only shows where the lamp
/// shines on it.
#[derive(Component, Debug)]
struct Mote {
    /// Random phases of its wandering.
    phase: Vec3,
}

impl Mote {
    fn spawn_haze(mut commands: Commands) {
        let room = Block::between(
            Vec3::new(-shop::SIZE.x / 2.0, 0.0, shop::FRONT),
            Vec3::new(shop::SIZE.x / 2.0, shop::SIZE.y, shop::FRONT + shop::SIZE.z),
        );
        commands.spawn((
            Name::new("shop haze"),
            FogVolume {
                fog_color: HAZE,
                density_factor: HAZE_DENSITY,
                light_intensity: HAZE_GLOW,
                ..default()
            },
            room.transform().with_scale(room.size),
        ));
    }

    fn spawn_cloud(
        add: On<Add<Lamp>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let mut rng = SmallRng::seed_from_u64(SEED);
        let mesh = meshes.add(Sphere::new(MOTE_RADIUS).mesh().uv(6, 4));
        // Adds the light it catches, so it vanishes in the dark.
        let material = materials.add(StandardMaterial {
            base_color: MOTE,
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Add,
            ..default()
        });
        commands.entity(add.entity).with_children(|lamp| {
            for _ in 0..MOTES {
                let at =
                    CLOUD.min() + CLOUD.size * Vec3::new(rng.random(), rng.random(), rng.random());
                lamp.spawn((
                    Mote {
                        phase: Vec3::new(rng.random(), rng.random(), rng.random()) * 100.0,
                    },
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(at),
                    NotShadowCaster,
                ));
            }
        });
    }

    /// Wanders slowly on the air currents, and comes back in on the other
    /// side when it leaves the cloud.
    fn drift(time: Res<Time>, mut motes: Query<(&Mote, &mut Transform)>) {
        let t = time.elapsed_secs();
        let dt = time.delta_secs();
        for (mote, mut transform) in &mut motes {
            let p = mote.phase;
            let wander = Vec3::new(
                (t * 0.23 + p.x).sin() + 0.5 * (t * 0.61 + p.y).sin(),
                (t * 0.19 + p.y).sin() + 0.5 * (t * 0.53 + p.z).sin(),
                (t * 0.29 + p.z).sin() + 0.5 * (t * 0.47 + p.x).sin(),
            );
            let moved = transform.translation + (wander * DRIFT + Vec3::Y * RISE) * dt;
            let min = CLOUD.min();
            transform.translation = min + (moved - min).rem_euclid(CLOUD.size);
        }
    }
}
