mod route;

use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt, rngs::SmallRng};

use self::route::{Route, Sight};
use super::super::{Kit, rng};
use super::models::Fleet;

const SEED: u64 = 9;
/// How far a flying car bobs up and down, in meters.
const HOVER_BOB: f32 = 0.25;
const CARS: usize = 22;
/// Seconds for a car to come out from behind a building, and its sound
/// with it.
const EMERGE: f32 = 2.0;
/// Time before a car sets off on its next route.
const PAUSE: Range<f32> = 0.0..25.0;

pub(super) struct FlyingPlugin;

impl Plugin for FlyingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SkyRng(rng(SEED))).add_systems(
            Update,
            (
                Sight::build.run_if(not(resource_exists::<Sight>)),
                SkyCar::fly.run_if(resource_exists::<Sight>),
            )
                .chain(),
        );
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

    /// From 0 to 1, how much of the car comes out from behind the buildings
    /// it starts and ends its route behind.
    pub(super) fn in_the_open(&self) -> f32 {
        self.route.as_ref().map_or(0.0, |route| {
            let left = route.duration - route.elapsed;
            (route.elapsed.min(left) / EMERGE).clamp(0.0, 1.0)
        })
    }

    /// Meters per second, while the car is on a route.
    pub(super) fn velocity(&self) -> Option<Vec3> {
        self.route.as_ref().map(|route| route.velocity)
    }

    fn fly(
        time: Res<Time>,
        sight: Res<Sight>,
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
                car.route = Route::random(&sight, &mut rng.0);
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
            if route.elapsed > route.duration {
                car.route = None;
                car.pause = rng.0.random_range(PAUSE);
                *visibility = Visibility::Hidden;
                continue;
            }
            let bob = (time.elapsed_secs() * 0.8 + car.phase).sin() * HOVER_BOB;
            transform.translation = route.at() + Vec3::Y * bob;
            // Local +X is the front.
            transform.rotation =
                Quat::from_rotation_y(f32::atan2(-route.velocity.z, route.velocity.x));
            // Shown from the very start of the route, which is out of sight.
            route.elapsed += dt;
        }
    }
}
