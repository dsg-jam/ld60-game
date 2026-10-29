mod draft;
mod flying;
mod models;
mod sound;
mod spray;

use std::error::Error;

use bevy::prelude::*;
use ld60_game_parts::liquid::Shake;
use ld60_geometry::surface::SurfaceMaterial;
use rand::RngExt;

pub use self::draft::Draft;
use self::flying::SkyCar;
use self::models::{Body, Fleet};
use self::spray::{Spray, SprayMaterial};
use super::street::{ROAD_PUDDLES, ROAD_SEED};
use super::wet::Wet;
use super::{CURB_HEIGHT, FAR_CURB, HALF_LENGTH, Kit, NEAR_CURB, rng, turned_to};

/// Millimeters of water on a soaked road.
const FILM: f32 = 1.0;
/// Millimeters of water in a puddle per unit [`Wet::puddle`] is above its
/// level.
const PUDDLE: f32 = 40.0;
/// Share of a puddle's water left for the rear wheels, after the front ones
/// pushed it aside.
const WAKE: f32 = 0.6;
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
        .init_resource::<Draft>()
        .add_systems(Startup, Vehicle::spawn_all)
        .add_systems(Update, (Vehicle::drive, Vehicle::rumble, Vehicle::push_air));
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
    body: Body,
}

impl Vehicle {
    /// Millimeters of water under each wheel of the vehicle at `transform`,
    /// in the order of [`Body::wheels`]. The front wheels push a puddle's
    /// water aside, so the rear ones get less of it.
    fn water(&self, transform: &GlobalTransform, wetness: f32) -> [f32; 4] {
        let wheels = self.body.wheels();
        std::array::from_fn(|index| {
            let at = transform.transform_point(wheels[index]).xz();
            let wake = if index < 2 { 1.0 } else { WAKE };
            let puddle = Wet::puddle(at, wetness, ROAD_PUDDLES, ROAD_SEED).max(0.0);
            FILM * wetness + PUDDLE * puddle * wake
        })
    }

    fn spawn_all(
        mut commands: Commands,
        kit: Res<Kit>,
        spray: Res<Spray>,
        mut sprays: ResMut<Assets<SprayMaterial>>,
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
                // Local +X is the front.
                let facing = turned_to(lane.velocity);
                let mut car = commands.spawn((
                    Name::new("car"),
                    Transform::from_xyz(x, lane.height, lane.z).with_rotation(facing),
                    Visibility::default(),
                ));
                let mut body = Body::Sedan;
                car.with_children(|parts| {
                    body = fleet.car(parts, &kit, &mut rng);
                    parts.spawn(spray.bundle(&mut sprays));
                });
                car.insert(Vehicle {
                    velocity: lane.velocity,
                    body,
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

    /// Passing vehicles push air into the shop.
    fn push_air(vehicles: Query<(&Vehicle, &Transform)>, mut draft: ResMut<Draft>) {
        let air = vehicles
            .iter()
            .map(|(vehicle, transform)| {
                Draft::from_vehicle(transform.translation, Vec3::X * vehicle.velocity)
            })
            .sum();
        draft.set_if_neq(Draft(air));
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
