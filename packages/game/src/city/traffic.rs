mod flying;
mod models;
mod sound;
mod spray;

use std::error::Error;

use bevy::prelude::*;
use rand::RngExt;

use self::flying::SkyCar;
use self::models::Fleet;
use self::spray::Spray;
use super::{CURB_HEIGHT, FAR_CURB, HALF_LENGTH, Kit, NEAR_CURB, rng, turned_to};
use crate::geometry::surface::SurfaceMaterial;
use crate::parts::liquid::Shake;

const SEED: u64 = 4;

/// All vehicles in a lane go the same speed, so they never catch up.
const LANES: [Lane; 2] = [
    Lane {
        height: -CURB_HEIGHT,
        z: NEAR_CURB - 2.4,
        velocity: 11.0,
        vehicles: 4,
    },
    Lane {
        height: -CURB_HEIGHT,
        z: FAR_CURB + 2.4,
        velocity: -13.0,
        vehicles: 4,
    },
];
/// Vehicles wrap around this far beyond the street ends.
const WRAP: f32 = 1.5;
/// How hard a vehicle shakes the ground at the shop: m/s² per m/s of speed,
/// divided by the squared distance plus [`RUMBLE_NEAR`].
const RUMBLE: f32 = 0.4;
const RUMBLE_NEAR: f32 = 4.0;

pub(super) struct TrafficPlugin;

impl Plugin for TrafficPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            spray::SprayPlugin,
            flying::FlyingPlugin,
            sound::TrafficSoundPlugin,
        ))
        .add_systems(Startup, Vehicle::spawn_all)
        .add_systems(Update, (Vehicle::drive, Vehicle::rumble));
    }
}

struct Lane {
    height: f32,
    z: f32,
    /// Along X.
    velocity: f32,
    vehicles: usize,
}

#[derive(Component, Debug)]
struct Vehicle {
    velocity: f32,
}

impl Vehicle {
    fn spawn_all(
        mut commands: Commands,
        kit: Res<Kit>,
        spray: Res<Spray>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let fleet = match Fleet::new(&mut meshes, &mut materials, &mut surfaces) {
            Ok(fleet) => fleet,
            Err(err) => {
                error!(error = &err as &dyn Error, "vehicle models do not build");
                return;
            }
        };
        let mut rng = rng(SEED);
        let span = 2.0 * HALF_LENGTH * WRAP;
        for lane in &LANES {
            let spacing = span / lane.vehicles as f32;
            for index in 0..lane.vehicles {
                let x = -span / 2.0 + spacing * (index as f32 + rng.random_range(0.0..0.6));
                let vehicle = Vehicle {
                    velocity: lane.velocity,
                };
                // Local +X is the front.
                let facing = turned_to(lane.velocity);
                commands
                    .spawn((
                        vehicle,
                        Transform::from_xyz(x, lane.height, lane.z).with_rotation(facing),
                        Visibility::default(),
                    ))
                    .with_children(|parts| {
                        fleet.car(parts, &kit, &mut rng);
                        parts.spawn(spray.bundle());
                    });
            }
        }
        SkyCar::spawn_all(&mut commands, &fleet, &kit, &mut rng);
    }

    /// Passing vehicles shake the ground at the shop, the harder the closer
    /// and faster they are.
    fn rumble(vehicles: Query<(&Vehicle, &Transform)>, mut shake: ResMut<Shake>) {
        for (vehicle, transform) in &vehicles {
            let distance = transform.translation.length_squared() + RUMBLE_NEAR;
            shake.0 += RUMBLE * vehicle.velocity.abs() / distance;
        }
    }

    fn drive(time: Res<Time>, mut vehicles: Query<(&Vehicle, &mut Transform)>) {
        let limit = HALF_LENGTH * WRAP;
        for (vehicle, mut transform) in &mut vehicles {
            let mut x = transform.translation.x + vehicle.velocity * time.delta_secs();
            if x > limit {
                x -= 2.0 * limit;
            } else if x < -limit {
                x += 2.0 * limit;
            }
            transform.translation.x = x;
        }
    }
}
