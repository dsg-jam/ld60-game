use std::f32::consts::TAU;
use std::ops::Range;

use bevy::{
    prelude::*,
    shape::{Aabb3d, RayCast3d},
};
use rand::{RngExt, rngs::SmallRng};

use super::super::super::buildings::Skyline;
use super::super::super::{FACADES, Segment};
use crate::menu::TITLE_EYES;
use crate::player::camera::PlayerCamera;

/// Routes cross the sky the player sees from the seat, through a point
/// this far to the side of straight ahead and this high up, in radians,
/// and this many meters away.
const VIEW_YAW: f32 = 0.7;
const VIEW_PITCH: Range<f32> = 0.05..1.0;
const VIEW_DISTANCE: Range<f32> = 40.0..400.0;
const ALTITUDE: Range<f32> = 18.0..160.0;
/// Routes end no lower than this.
const LOWEST: f32 = 10.0;
const SPEED: Range<f32> = 12.0..45.0;
/// Meters of climb or descent per meter flown.
const CLIMB: Range<f32> = -0.06..0.06;
/// Routes keep this far away from buildings.
const CLEARANCE: f32 = 8.0;
const TRIES: usize = 10;
const HEADINGS: usize = 6;
/// Half length of the stretch of street that no route flies over.
const STREET_HALF_LENGTH: f32 = 140.0;
/// Margin beyond the facades and behind the shop.
const STREET_MARGIN: f32 = 12.0;
/// Routes end short of where the camera stops drawing.
const FARTHEST: f32 = 950.0;
/// Meters between the spots tried for the ends of a route.
const STEP: f32 = 15.0;
/// A car with its bobbing, and the step it takes in a frame, fits in a
/// cube this many meters to each side of its middle.
const CAR_SIZE: f32 = 4.0;
/// The player's head moves this far from the seat to look at things on the
/// desk.
const DESK_REACH: Vec3 = Vec3::new(0.5, 0.4, 0.5);

/// A straight flight at a steady speed.
#[derive(Debug)]
pub(super) struct Route {
    pub(super) start: Vec3,
    pub(super) velocity: Vec3,
    pub(super) elapsed: f32,
    pub(super) duration: f32,
}

/// Where the player may look from, and what blocks the view, so cars come
/// and go out of sight.
#[derive(Resource, Debug)]
pub(super) struct Sight {
    seat: Vec3,
    /// The player's eyes are always somewhere between these: on the title
    /// screen, at the desk, or on the way from one to the other.
    eyes: Vec<Vec3>,
    walls: Vec<Aabb3d>,
    /// The buildings, as far larger as routes keep away from them.
    keep_out: Vec<Aabb3d>,
}

impl Route {
    /// A straight route through the sky the player sees, which keeps clear
    /// of every building and starts and ends behind them, if one turns up
    /// within a few tries.
    pub(super) fn random(sight: &Sight, rng: &mut SmallRng) -> Option<Self> {
        (0..TRIES).find_map(|_| {
            let over = sight.open_sky(rng)?;
            let (start, end) = (0..HEADINGS).find_map(|_| {
                let heading = rng.random_range(0.0..TAU);
                let along = Vec3::new(heading.cos(), rng.random_range(CLIMB), heading.sin());
                let along = Dir3::new(along).ok()?;
                Some((sight.end(over, -along)?, sight.end(over, along)?))
            })?;
            let speed = rng.random_range(SPEED);
            let duration = start.distance(end) / speed;
            Some(Self {
                start,
                velocity: (end - start) / duration,
                elapsed: 0.0,
                duration,
            })
        })
    }

    /// Where the car is.
    pub(super) fn at(&self) -> Vec3 {
        self.start + self.velocity * self.elapsed
    }
}

impl Sight {
    pub(super) fn build(
        skyline: Res<Skyline>,
        camera: Single<&PlayerCamera>,
        mut commands: Commands,
    ) {
        let seat = camera.seat().translation;
        let desk = Self::corners(DESK_REACH).map(|corner| seat + corner);
        let eyes = TITLE_EYES.into_iter().chain(desk).collect();
        let walls = skyline
            .volumes
            .iter()
            .map(|block| Aabb3d::new(block.center, block.size / 2.0))
            .collect();
        let mut keep_out: Vec<_> = skyline
            .volumes
            .iter()
            .map(|block| Aabb3d::new(block.center, block.size / 2.0 + CLEARANCE))
            .collect();
        keep_out.push(Self::over_street());
        commands.insert_resource(Self {
            seat,
            eyes,
            walls,
            keep_out,
        });
    }

    /// The sky above the street in front of the shop.
    fn over_street() -> Aabb3d {
        let lo = Vec3::new(-STREET_HALF_LENGTH, 0.0, FACADES - STREET_MARGIN);
        let hi = Vec3::new(STREET_HALF_LENGTH, 2.0 * ALTITUDE.end, STREET_MARGIN);
        Aabb3d::new((lo + hi) / 2.0, (hi - lo) / 2.0)
    }

    /// A random point in the sky that the player sees from the seat.
    fn open_sky(&self, rng: &mut SmallRng) -> Option<Vec3> {
        let yaw = rng.random_range(-VIEW_YAW..VIEW_YAW);
        let pitch = rng.random_range(VIEW_PITCH);
        let look = Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            -yaw.cos() * pitch.cos(),
        );
        let at = self.seat + look * rng.random_range(VIEW_DISTANCE);
        let sight = Segment {
            from: self.seat,
            to: at,
        };
        (ALTITUDE.contains(&at.y) && self.walls.iter().all(|wall| sight.misses(wall))).then_some(at)
    }

    /// The first spot from `over` towards `along` where a car is out of
    /// sight from everywhere the player may look from, unless a building
    /// is in the way before it.
    fn end(&self, over: Vec3, along: Dir3) -> Option<Vec3> {
        let clear = self
            .keep_out
            .iter()
            .filter_map(|bounds| RayCast3d::new(over, along, FARTHEST).aabb_intersection_at(bounds))
            .fold(FARTHEST, f32::min);
        (1..)
            .map(|step| step as f32 * STEP)
            .take_while(|&distance| distance < clear)
            .map(|distance| over + along * distance)
            .take_while(|at| at.y > LOWEST && self.farthest(*at) < FARTHEST)
            .find(|&at| self.hidden(at))
    }

    /// Meters from `at` to the eye farthest from it.
    fn farthest(&self, at: Vec3) -> f32 {
        self.eyes
            .iter()
            .map(|eye| eye.distance(at))
            .fold(0.0, f32::max)
    }

    /// Whether a car at `at` is hidden whole behind one building from
    /// every eye. Seen from any of the eyes, the building covers the corners
    /// of a cube around the car, so it also covers the cube from anywhere
    /// between the eyes.
    fn hidden(&self, at: Vec3) -> bool {
        let car = Self::corners(Vec3::splat(CAR_SIZE));
        self.walls.iter().any(|wall| {
            self.eyes.iter().all(|&eye| {
                car.iter().all(|&corner| {
                    let sight = Segment {
                        from: eye,
                        to: at + corner,
                    };
                    !sight.misses(wall)
                })
            })
        })
    }

    /// The corners of a box `reach` to each side of the origin.
    fn corners(reach: Vec3) -> [Vec3; 8] {
        std::array::from_fn(|corner| {
            let side = |bit: usize| if corner & bit == 0 { -1.0 } else { 1.0 };
            reach * Vec3::new(side(1), side(2), side(4))
        })
    }
}
