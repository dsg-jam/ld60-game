mod models;
mod sky;
mod spray;

use std::error::Error;

use bevy::prelude::*;
use rand::RngExt;

use self::models::Fleet;
use self::sky::SkyCar;
use self::spray::Spray;
use super::{CURB_HEIGHT, FAR_CURB, HALF_LENGTH, Kit, NEAR_CURB, rng};
use crate::geometry::surface::SurfaceMaterial;

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
const HOVER_BOB: f32 = 0.25;

pub(super) struct TrafficPlugin;

impl Plugin for TrafficPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((spray::SprayPlugin, sky::SkyPlugin))
            .add_systems(Startup, Vehicle::spawn_all)
            .add_systems(Update, Vehicle::drive);
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
                let facing = if lane.velocity > 0.0 {
                    Quat::IDENTITY
                } else {
                    Quat::from_rotation_y(std::f32::consts::PI)
                };
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
