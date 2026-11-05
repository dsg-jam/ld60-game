use std::f32::consts::{PI, TAU};
use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::super::bird::Bird;
use super::super::sound::Voice;
use super::super::{SITTER_TALK, Stool, WALKER_TALK};
use super::{ACCELERATION, Walker};

/// People feel like sitting down after walking this many meters, and
/// then look for a free seat over this many more before they give up.
const SEAT_EVERY: Range<f32> = 20.0..70.0;
const SEEK_FOR: f32 = 40.0;
/// A seat is just ahead between these many meters.
const SEAT_AHEAD: Range<f32> = 1.0..6.0;
/// Seats this far beyond the sidewalk count as on it.
const SEAT_REACH: f32 = 1.0;
/// People stop beside a seat this far from its middle and slide onto it.
const BESIDE: f32 = 0.38;
/// They slow down this close to where they go.
const SLOW_DOWN: f32 = 0.8;
/// Seconds to sit down or get up, and to stay seated.
const SIT_TIME: f32 = 1.8;
const SIT_FOR: Range<f32> = 25.0..120.0;
/// Seated, people turn this far from the counter, to the side.
const SEAT_TURN: f32 = 0.6;
/// How fast people turn, per second.
const TURN_RATE: f32 = 4.0;

/// What a local is doing on its outing.
#[derive(Debug, Clone, Copy)]
pub(super) enum Activity {
    /// Walking along the sidewalk, now and then stopping to look.
    Stroll,
    /// Walking to the side of a stool, to sit on it from there. The stool
    /// is the one at its index in the places, `side` is -1 or 1 along the
    /// stool's X.
    ToSeat { stool: usize, side: f32 },
    /// On a stool: sat down as far as `sitting`, turned this far from the
    /// counter, and staying for `left` seconds more before it gets up.
    Seated {
        stool: usize,
        side: f32,
        sitting: f32,
        turn: f32,
        left: f32,
        rising: bool,
    },
}

/// Which stools someone sits on or is heading for, in the order of the
/// places.
#[derive(Resource, Debug, Default)]
pub(in super::super) struct Seats(Vec<bool>);

impl Seats {
    pub(in super::super) fn new(stools: usize) -> Self {
        Self(vec![false; stools])
    }

    fn claim(&mut self, stool: usize) {
        self.0[stool] = true;
    }

    fn free(&mut self, stool: usize) {
        self.0[stool] = false;
    }

    fn is_free(&self, stool: usize) -> bool {
        !self.0[stool]
    }
}

impl Activity {
    pub(super) fn is_stroll(self) -> bool {
        matches!(self, Self::Stroll)
    }
}

impl Walker {
    /// `walker` already sitting on `stool`, at its index `index`.
    pub(in super::super) fn seated(
        mut walker: Self,
        index: usize,
        stool: &Stool,
        seats: &mut Seats,
        rng: &mut impl Rng,
    ) -> Self {
        seats.claim(index);
        let at = stool.at.translation;
        walker.x = at.x;
        walker.z = at.z;
        walker.activity = Activity::Seated {
            stool: index,
            side: if rng.random_bool(0.5) { 1.0 } else { -1.0 },
            sitting: 1.0,
            turn: rng.random_range(-SEAT_TURN..SEAT_TURN),
            left: rng.random_range(SIT_FOR),
            rising: false,
        };
        walker.yaw = facing(stool);
        walker
    }

    /// Whether a seat at `at` lies on its sidewalk.
    pub(super) fn reaches(&self, at: Vec3) -> bool {
        let room = &self.sidewalk.room;
        at.z > room.start - SEAT_REACH && at.z < room.end + SEAT_REACH
    }

    /// Walking, it feels like sitting down now and then, and then heads
    /// for the next free seat just ahead.
    pub(super) fn look_for_seat(
        &mut self,
        bird: &Bird,
        stools: &[Stool],
        seats: &mut Seats,
        raining: bool,
        dt: f32,
        rng: &mut impl Rng,
    ) {
        self.until_seat -= bird.speed * dt;
        if self.until_seat < -SEEK_FOR {
            self.until_seat = rng.random_range(SEAT_EVERY);
        }
        if self.until_seat > 0.0 || self.stopped > 0.0 {
            return;
        }
        let dry = !raining || bird.has_umbrella_open();
        let seat = stools
            .iter()
            .enumerate()
            .filter(|&(index, stool)| {
                seats.is_free(index) && self.reaches(stool.at.translation) && (dry || stool.covered)
            })
            .map(|(index, stool)| {
                // Come at it from the side it is approached from.
                let across = stool.at.rotation * Vec3::X;
                let side = -(across.x * self.heading).signum();
                (index, side, (beside(stool, side).x - self.x) * self.heading)
            })
            .filter(|(.., ahead)| SEAT_AHEAD.contains(ahead))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        if let Some((stool, side, _)) = seat {
            seats.claim(stool);
            self.activity = Activity::ToSeat { stool, side };
        }
    }

    /// Walks to the side of the stool and then sits on it, stays, gets up
    /// and walks on.
    pub(super) fn go_out(
        &mut self,
        bird: &mut Bird,
        voice: &mut Voice,
        stools: &[Stool],
        seats: &mut Seats,
        dt: f32,
        rng: &mut impl Rng,
    ) {
        match self.activity {
            Activity::Stroll => {}
            Activity::ToSeat { stool: index, side } => {
                let stool = &stools[index];
                if self.walk_to(bird, beside(stool, side), dt) {
                    self.activity = Activity::Seated {
                        stool: index,
                        side,
                        sitting: 0.0,
                        turn: rng.random_range(-SEAT_TURN..SEAT_TURN),
                        left: rng.random_range(SIT_FOR),
                        rising: false,
                    };
                }
            }
            Activity::Seated {
                stool: index,
                side,
                mut sitting,
                turn,
                mut left,
                mut rising,
            } => {
                let stool = &stools[index];
                bird.speed = 0.0;
                let (_, spring, _) = Bird::hop(sitting);
                let facing_seat = facing(stool) + turn * ease(spring);
                let ready = (facing_seat - self.yaw + PI).rem_euclid(TAU) - PI;
                if rising {
                    sitting -= dt / SIT_TIME;
                } else if sitting == 0.0 && ready.abs() > 0.08 {
                    // Turns to the counter before it hops on.
                } else if sitting < 1.0 {
                    sitting += dt / SIT_TIME;
                } else {
                    voice.talk_every(SITTER_TALK);
                    left -= dt;
                    rising = left <= 0.0;
                }
                sitting = sitting.clamp(0.0, 1.0);
                let (_, spring, _) = Bird::hop(sitting);
                let onto = ease(spring);
                let at = beside(stool, side).lerp(stool.at.translation.xz(), onto);
                self.x = at.x;
                self.z = at.y;
                self.yaw = turned(self.yaw, facing(stool) + turn * onto, TURN_RATE * dt);
                bird.sit(stool.height, sitting, stool.covered);
                if rising && sitting <= 0.0 {
                    seats.free(index);
                    bird.sit(stool.height, 0.0, false);
                    voice.talk_every(WALKER_TALK);
                    let heading = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                    self.walk_on(heading, rng);
                } else {
                    self.activity = Activity::Seated {
                        stool: index,
                        side,
                        sitting,
                        turn,
                        left,
                        rising,
                    };
                }
            }
        }
    }

    /// Steps towards `to`, slowing down as it gets there. Returns whether
    /// it is there.
    fn walk_to(&mut self, bird: &mut Bird, to: Vec2, dt: f32) -> bool {
        let way = to - Vec2::new(self.x, self.z);
        let distance = way.length();
        let target = bird.pace * (distance / SLOW_DOWN).min(1.0);
        let change = ACCELERATION * dt;
        bird.speed += (target - bird.speed).clamp(-change, change);
        let step = way.normalize_or_zero() * (bird.speed * dt).min(distance);
        self.x += step.x;
        self.z += step.y;
        if way.x.abs() > 0.05 {
            self.heading = way.x.signum();
        }
        if step.length() > 1e-4 {
            self.yaw = turned(self.yaw, step.x.atan2(step.y), TURN_RATE * dt);
        }
        distance < 0.04
    }

    /// Strolling along the sidewalk towards `heading`.
    pub(super) fn walk_on(&mut self, heading: f32, rng: &mut impl Rng) {
        self.heading = heading;
        self.wish = rng.random_range(self.lanes());
        self.lane = self.wish;
        self.until_seat = rng.random_range(SEAT_EVERY);
        self.activity = Activity::Stroll;
    }
}

/// Where a stool's sitter stands before sliding onto it.
fn beside(stool: &Stool, side: f32) -> Vec2 {
    stool.at.transform_point(Vec3::X * side * BESIDE).xz()
}

/// Which way a stool faces, as a yaw.
fn facing(stool: &Stool) -> f32 {
    let ahead = stool.at.rotation * Vec3::Z;
    ahead.x.atan2(ahead.z)
}

/// `from` turned towards `to` by up to `step`, the short way round.
fn turned(from: f32, to: f32, step: f32) -> f32 {
    let difference = (to - from + PI).rem_euclid(TAU) - PI;
    from + difference.clamp(-step, step)
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
