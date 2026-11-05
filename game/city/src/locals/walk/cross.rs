use std::f32::consts::PI;
use std::ops::Range;
use std::ptr;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::super::super::traffic::Vehicle;
use super::super::super::{CURB_HEIGHT, FAR_CURB, NEAR_CURB};
use super::super::bird::Bird;
use super::super::gaze::Gaze;
use super::outing::Activity;
use super::{SIDEWALKS, Sidewalk, Walker};

/// People cross the street after walking this many meters, where it can
/// be seen from the shop.
pub(super) const CROSS_EVERY: Range<f32> = 80.0..250.0;
const CROSS_WITHIN: f32 = 35.0;
/// Crossing, people walk this much faster than they stroll.
const HURRY: f32 = 1.5;
/// A vehicle takes up this much of the road to each side of its lane,
/// and people keep this much room to it along the street.
const LANE_HALF: f32 = 1.4;
const ROOM: f32 = 2.0;
/// At the curb, people look one way and the other for this many seconds
/// each, and wait at least this long.
const LOOK_EACH: Range<f32> = 0.7..1.4;
const WAIT_AT_LEAST: f32 = 1.8;
/// The ground steps down to the road over this much.
const STEP_DOWN: f32 = 0.25;

/// How far along crossing the street someone is.
#[derive(Debug, Clone, Copy)]
pub(super) enum Crossing {
    ToCurb,
    /// At the curb, looking out for vehicles, to the side of `side`.
    Waiting {
        waited: f32,
        look: f32,
        side: f32,
    },
    Across,
}

/// The vehicles on the road and where they are now.
pub(super) type Cars<'a> = [(&'a Vehicle, Vec3)];

impl Walker {
    /// Heads for the curb to cross the street, if it is near the shop.
    /// Returns whether it does.
    pub(super) fn start_crossing(&mut self, rng: &mut impl Rng) -> bool {
        self.until_cross = rng.random_range(CROSS_EVERY);
        if self.x.abs() > CROSS_WITHIN {
            return false;
        }
        self.activity = Activity::Cross {
            x: self.x + self.heading * 1.5,
            step: Crossing::ToCurb,
        };
        true
    }

    /// Walks to the curb, looks both ways until the road is clear, then
    /// crosses to the other sidewalk and strolls on there.
    pub(super) fn cross(
        &mut self,
        bird: &mut Bird,
        gaze: &mut Gaze,
        cars: &Cars,
        dt: f32,
        rng: &mut impl Rng,
    ) {
        let Activity::Cross { x, step } = self.activity else {
            return;
        };
        let other = self.other_side();
        let pace = bird.pace;
        let over = (other.curb - self.sidewalk.curb).signum();
        let step = match step {
            Crossing::ToCurb => {
                if self.walk_to(bird, Vec2::new(x, self.sidewalk.curb), pace, dt) {
                    Crossing::Waiting {
                        waited: 0.0,
                        look: 0.0,
                        side: if rng.random_bool(0.5) { 1.0 } else { -1.0 },
                    }
                } else {
                    Crossing::ToCurb
                }
            }
            Crossing::Waiting {
                mut waited,
                mut look,
                mut side,
            } => {
                bird.speed = 0.0;
                let across = if over > 0.0 { 0.0 } else { PI };
                self.yaw = super::outing::turned(self.yaw, across, 4.0 * dt);
                waited += dt;
                look -= dt;
                if look <= 0.0 {
                    look = rng.random_range(LOOK_EACH);
                    side = -side;
                    gaze.glance_at(Vec2::new(side * 1.2, 0.05), look);
                }
                let speed = pace * HURRY;
                if waited >= WAIT_AT_LEAST && self.clear(x, other.curb, speed, cars) {
                    Crossing::Across
                } else {
                    Crossing::Waiting { waited, look, side }
                }
            }
            Crossing::Across => {
                if self.walk_to(bird, Vec2::new(x, other.curb), pace * HURRY, dt) {
                    self.sidewalk = other;
                    let heading = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                    self.walk_on(heading, rng);
                    return;
                }
                Crossing::Across
            }
        };
        self.activity = Activity::Cross { x, step };
    }

    fn other_side(&self) -> &'static Sidewalk {
        SIDEWALKS
            .iter()
            .find(|sidewalk| !ptr::eq(*sidewalk, self.sidewalk))
            .unwrap_or(self.sidewalk)
    }

    /// Whether it can walk from where it is at `x` over to `to` at `speed`
    /// without any of the `cars` coming near while it is in their lane.
    fn clear(&self, x: f32, to: f32, speed: f32, cars: &Cars) -> bool {
        let from = self.z;
        let (near, far) = (from.min(to), from.max(to));
        cars.iter().all(|&(car, at)| {
            let lane = at.z - LANE_HALF..at.z + LANE_HALF;
            if lane.end < near || lane.start > far {
                return true;
            }
            let [a, b] = [lane.start, lane.end].map(|z| (z - from).abs() / speed);
            // It starts from standing, so it may be a little late.
            !car.passes(at, x, ROOM, a.min(b) - 0.3..a.max(b) + 1.2)
        })
    }
}

/// The height of the ground at `z`: the sidewalks, or the road between
/// the curbs.
pub(super) fn ground(z: f32) -> f32 {
    let inside = (NEAR_CURB - z).min(z - FAR_CURB);
    -CURB_HEIGHT * (inside / STEP_DOWN).clamp(0.0, 1.0)
}
