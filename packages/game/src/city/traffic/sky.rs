use std::ops::Range;

use bevy::{
    prelude::*,
    shape::{Aabb3d, RayCast3d},
};
use rand::{Rng, RngExt, rngs::SmallRng};

use super::super::buildings::Skyline;
use super::super::{FACADES, Kit, rng};
use super::HOVER_BOB;
use super::models::Fleet;
use crate::geometry::block::Block;

const SEED: u64 = 5;
const CARS: usize = 22;
/// Routes pass over a random point in this area, beyond the street.
const AREA_X: Range<f32> = -160.0..160.0;
const AREA_Z: Range<f32> = -240.0..-20.0;
const ALTITUDE: Range<f32> = 18.0..140.0;
/// Cars start and end this far from the point their route passes over.
const REACH: f32 = 220.0;
const SPEED: Range<f32> = 12.0..45.0;
/// Climb or descent over a whole route.
const CLIMB: Range<f32> = -25.0..25.0;
/// Routes keep this far away from buildings.
const CLEARANCE: f32 = 8.0;
const TRIES: usize = 30;
/// Half length of the stretch of street that no route flies over.
const STREET_HALF_LENGTH: f32 = 140.0;
/// Margin beyond the facades and behind the shop.
const STREET_MARGIN: f32 = 12.0;
/// Time before a car sets off on its next route.
const PAUSE: Range<f32> = 0.0..25.0;

pub(super) struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SkyRng(rng(SEED)))
            .add_systems(Update, SkyCar::fly);
    }
}

#[derive(Resource)]
struct SkyRng(SmallRng);

/// A flying car crossing the sky on routes of its own, in any direction.
#[derive(Component, Debug, Default)]
#[require(Transform, Visibility)]
pub(super) struct SkyCar {
    route: Option<Route>,
    /// Seconds left before the next route.
    pause: f32,
    /// The first route starts somewhere along the way, so the sky isn't
    /// empty at the start.
    started: bool,
    phase: f32,
}

#[derive(Debug)]
struct Route {
    start: Vec3,
    velocity: Vec3,
    elapsed: f32,
    duration: f32,
}

impl SkyCar {
    pub(super) fn spawn_all(commands: &mut Commands, fleet: &Fleet, kit: &Kit, rng: &mut impl Rng) {
        for _ in 0..CARS {
            commands
                .spawn((
                    SkyCar {
                        phase: rng.random_range(0.0..10.0),
                        ..default()
                    },
                    Visibility::Hidden,
                ))
                .with_children(|parts| fleet.hover(parts, kit, rng));
        }
    }

    fn fly(
        time: Res<Time>,
        skyline: Res<Skyline>,
        mut rng: ResMut<SkyRng>,
        mut cars: Query<(&mut SkyCar, &mut Transform, &mut Visibility)>,
    ) {
        let dt = time.delta_secs();
        for (mut car, mut transform, mut visibility) in &mut cars {
            let car = &mut *car;
            if car.route.is_none() {
                car.pause -= dt;
                if car.pause > 0.0 {
                    continue;
                }
                car.route = Route::random(&skyline.volumes, &mut rng.0);
                let Some(route) = &mut car.route else {
                    car.pause = rng.0.random_range(PAUSE);
                    continue;
                };
                if !car.started {
                    route.elapsed = rng.0.random_range(0.0..route.duration);
                    car.started = true;
                }
                *visibility = Visibility::Inherited;
            }
            let Some(route) = &mut car.route else {
                continue;
            };
            route.elapsed += dt;
            if route.elapsed > route.duration {
                car.route = None;
                car.pause = rng.0.random_range(PAUSE);
                *visibility = Visibility::Hidden;
                continue;
            }
            let bob = (time.elapsed_secs() * 0.8 + car.phase).sin() * HOVER_BOB;
            transform.translation = route.start + route.velocity * route.elapsed + Vec3::Y * bob;
            // Local +X is the front.
            transform.rotation =
                Quat::from_rotation_y(f32::atan2(-route.velocity.z, route.velocity.x));
        }
    }
}

impl Route {
    /// A straight route that keeps clear of every building, if one turns up
    /// within a few tries.
    fn random(volumes: &[Block], rng: &mut SmallRng) -> Option<Self> {
        (0..TRIES).find_map(|_| {
            let over = Vec3::new(
                rng.random_range(AREA_X),
                rng.random_range(ALTITUDE),
                rng.random_range(AREA_Z),
            );
            let heading = rng.random_range(0.0..std::f32::consts::TAU);
            let across = Vec3::new(heading.cos(), 0.0, heading.sin()) * REACH;
            let climb = Vec3::Y * rng.random_range(CLIMB) / 2.0;
            let start = over - across - climb;
            let end = over + across + climb;
            let street = Self::over_street();
            let clear = Self::misses(start, end, &street)
                && volumes.iter().all(|block| {
                    let bounds = Aabb3d::new(block.center, block.size / 2.0 + CLEARANCE);
                    Self::misses(start, end, &bounds)
                });
            clear.then(|| {
                let speed = rng.random_range(SPEED);
                let duration = start.distance(end) / speed;
                Self {
                    start,
                    velocity: (end - start) / duration,
                    elapsed: 0.0,
                    duration,
                }
            })
        })
    }

    /// The sky above the street in front of the shop.
    fn over_street() -> Aabb3d {
        let lo = Vec3::new(-STREET_HALF_LENGTH, 0.0, FACADES - STREET_MARGIN);
        let hi = Vec3::new(STREET_HALF_LENGTH, 2.0 * ALTITUDE.end, STREET_MARGIN);
        Aabb3d::new((lo + hi) / 2.0, (hi - lo) / 2.0)
    }

    fn misses(start: Vec3, end: Vec3, bounds: &Aabb3d) -> bool {
        let Ok(direction) = Dir3::new(end - start) else {
            return true;
        };
        let ray = RayCast3d::new(start, direction, start.distance(end));
        ray.aabb_intersection_at(bounds).is_none()
    }
}
