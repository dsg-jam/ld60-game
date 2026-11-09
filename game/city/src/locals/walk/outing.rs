use std::f32::consts::{PI, TAU};
use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::super::bird::Bird;
use super::super::gaze::Gaze;
use super::super::sound::Voice;
use super::super::{Places, SITTER_TALK, Spot, WALKER_TALK};
use super::chat::{CHAT_TALK, NEIGHBOUR, TURN_EVERY, TURN_FOR};
use super::cross::{Cars, Crossing};
use super::meal::Dining;
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
pub(super) const BESIDE: f32 = 0.38;
/// They slow down this close to where they go.
const SLOW_DOWN: f32 = 0.8;
/// Seconds to sit down or get up, and to stay seated.
pub(super) const SIT_TIME: f32 = 1.8;
const SIT_FOR: Range<f32> = 25.0..120.0;
/// Seated, people turn this far from the counter, to the side.
const SEAT_TURN: f32 = 0.6;
/// How fast people turn, per second.
const TURN_RATE: f32 = 4.0;
/// At a counter, people look over the goods for this many seconds, at
/// one thing after another for these many, this far to the side and down.
const BROWSE_FOR: Range<f32> = 5.0..16.0;
const LOOK_FOR: Range<f32> = 1.2..3.0;
const LOOK_ASIDE: f32 = 0.45;
const LOOK_DOWN: Range<f32> = 0.3..0.6;
/// Looking over the goods, people lean in this far.
const LEAN_IN: f32 = 0.18;
/// Caught in the rain, people wait under an awning for at most this many
/// seconds, looking somewhere new every this many, and walk on once the
/// rain has eased for this many.
const SHELTER_FOR: Range<f32> = 40.0..150.0;
const SHELTER_LOOK: Range<f32> = 1.5..4.5;
const EASED_FOR: f32 = 3.0;
/// Having given up waiting, people walk this many meters before they
/// shelter again.
const SHELTER_AGAIN: Range<f32> = 40.0..100.0;
/// Friends sit down together on stools this close to each other, this far
/// from where they chatted at most.
const SIDE_BY_SIDE: f32 = 2.0;
const TOGETHER_WITHIN: f32 = 8.0;

/// What a local is doing on its outing.
#[derive(Debug, Clone, Copy)]
pub(super) enum Activity {
    /// Walking along the sidewalk, now and then stopping to look.
    Stroll,
    /// Walking to the side of a stool, to sit on it from there. The stool
    /// is the one at its index in the places, `side` is -1 or 1 along the
    /// stool's X.
    ToSeat { stool: usize, side: f32 },
    /// Standing to chat with someone at `with` for `left` seconds more,
    /// before going to sit on a stool from a side, if it has one, or
    /// walking on towards `heading`.
    Chat {
        with: Vec2,
        left: f32,
        heading: f32,
        then: Option<(usize, f32)>,
    },
    /// Walking to a spot at a counter and looking over the goods there for
    /// `left` seconds more, at one thing until `look` runs out.
    Browse {
        spot: usize,
        arrived: bool,
        left: f32,
        look: f32,
    },
    /// Crossing the street at `x`.
    Cross { x: f32, step: Crossing },
    /// On an errand to the shop counter, see [`Visitor`](super::visit::Visitor).
    Visit,
    /// Walking to a spot under an awning and waiting there for the rain
    /// to ease, for `left` seconds at most, looking around until `look`
    /// runs out. `eased` counts the seconds the rain has been light.
    Shelter {
        spot: usize,
        arrived: bool,
        left: f32,
        look: f32,
        eased: f32,
    },
    /// On a stool: sat down as far as `sitting`, turned this far from the
    /// counter, having its meal, and staying for `left` seconds more
    /// before it gets up.
    Seated {
        stool: usize,
        side: f32,
        sitting: f32,
        turn: f32,
        dining: Dining,
        left: f32,
        rising: bool,
        /// Seconds until it turns to a neighbour again.
        glance: f32,
    },
}

/// Which stools and spots at the counters someone is at or heading for,
/// in the order of the places.
#[derive(Resource, Debug, Default)]
pub(in super::super) struct Seats {
    stools: Vec<bool>,
    standing: Vec<bool>,
    awnings: Vec<bool>,
}

/// Where the locals buying or eating at the stalls are, for the keepers.
#[derive(Resource, Debug, Default)]
pub(in super::super) struct Customers(pub(in super::super) Vec<Vec3>);

/// What an outing sees around it this frame.
pub(super) struct Around<'a> {
    pub(super) places: &'a Places,
    /// Where the customers at the stalls were last frame.
    pub(super) customers: &'a [Vec3],
    pub(super) cars: &'a Cars<'a>,
    /// Whether it rains hard enough to want shelter.
    pub(super) raining: bool,
}

impl Seats {
    pub(in super::super) fn new(places: &Places) -> Self {
        Self {
            stools: vec![false; places.stools.len()],
            standing: vec![false; places.standing.len()],
            awnings: vec![false; places.awnings.len()],
        }
    }

    /// Claims two free stools side by side near two people at `at`, out of
    /// the rain unless `dry`, and only those `reaches` allows. Returns for
    /// each person its stool and the side it comes at it from, the outer
    /// one, so that they do not cross.
    pub(super) fn side_by_side(
        &mut self,
        stools: &[Spot],
        at: [Vec2; 2],
        dry: bool,
        reaches: impl Fn(Vec3) -> bool,
    ) -> Option<[(usize, f32); 2]> {
        let middle = (at[0] + at[1]) / 2.0;
        let free = |index: usize| {
            let stool = &stools[index];
            !self.stools[index] && (dry || stool.covered) && reaches(stool.at.translation)
        };
        let (a, b, _) = (0..stools.len())
            .filter(|&a| free(a))
            .flat_map(|a| ((a + 1)..stools.len()).map(move |b| (a, b)))
            .filter(|&(a, b)| {
                free(b)
                    && stools[a].at.translation.distance(stools[b].at.translation) < SIDE_BY_SIDE
            })
            .map(|(a, b)| {
                let between = (stools[a].at.translation + stools[b].at.translation).xz() / 2.0;
                (a, b, between.distance(middle))
            })
            .filter(|&(.., away)| away < TOGETHER_WITHIN)
            .min_by(|x, y| x.2.total_cmp(&y.2))?;
        self.stools[a] = true;
        self.stools[b] = true;
        let across = stools[a].at.rotation * Vec3::X;
        let along = |point: Vec2| point.dot(across.xz());
        let [low, high] =
            if along(stools[a].at.translation.xz()) < along(stools[b].at.translation.xz()) {
                [a, b]
            } else {
                [b, a]
            };
        let (first, second) = if along(at[0]) < along(at[1]) {
            (low, high)
        } else {
            (high, low)
        };
        let outer = |stool| (stool, if stool == low { -1.0 } else { 1.0 });
        Some([outer(first), outer(second)])
    }
}

impl Activity {
    /// Whether it is at a stall, buying or eating there.
    pub(super) fn is_customer(self) -> bool {
        match self {
            Self::Browse { arrived, .. } => arrived,
            Self::Seated { sitting, .. } => sitting > 0.5,
            Self::Stroll
            | Self::ToSeat { .. }
            | Self::Chat { .. }
            | Self::Shelter { .. }
            | Self::Cross { .. }
            | Self::Visit => false,
        }
    }

    pub(super) fn is_stroll(self) -> bool {
        matches!(self, Self::Stroll)
    }

    pub(super) fn is_visit(self) -> bool {
        matches!(self, Self::Visit)
    }
}

impl Walker {
    /// `walker` already sitting on `stool`, at its index `index`.
    pub(in super::super) fn seated(
        mut walker: Self,
        index: usize,
        stool: &Spot,
        seats: &mut Seats,
        rng: &mut impl Rng,
    ) -> Self {
        seats.stools[index] = true;
        let at = stool.at.translation;
        walker.x = at.x;
        walker.z = at.z;
        walker.activity = Activity::Seated {
            stool: index,
            side: if rng.random_bool(0.5) { 1.0 } else { -1.0 },
            sitting: 1.0,
            turn: rng.random_range(-SEAT_TURN..SEAT_TURN),
            dining: Dining::served(false, rng),
            left: rng.random_range(SIT_FOR),
            rising: false,
            glance: rng.random_range(TURN_EVERY),
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
        places: &Places,
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
        let seat = self.free_ahead(&places.stools, &seats.stools, dry, |stool| {
            // Come at it from the side it is approached from.
            let across = stool.at.rotation * Vec3::X;
            let side = -(across.x * self.heading).signum();
            (side, beside(stool, side))
        });
        if let Some((stool, side)) = seat {
            seats.stools[stool] = true;
            self.activity = Activity::ToSeat { stool, side };
        }
    }

    /// Instead of stopping on the sidewalk, it steps up to a free spot at a
    /// counter just ahead, if there is one, to look over the goods there.
    /// Returns whether it does.
    pub(super) fn browse(
        &mut self,
        bird: &Bird,
        places: &Places,
        seats: &mut Seats,
        raining: bool,
        rng: &mut impl Rng,
    ) -> bool {
        let dry = !raining || bird.has_umbrella_open();
        let spot = self.free_ahead(&places.standing, &seats.standing, dry, |spot| {
            ((), spot.at.translation.xz())
        });
        if let Some((spot, ())) = spot {
            seats.standing[spot] = true;
            self.activity = Activity::Browse {
                spot,
                arrived: false,
                left: rng.random_range(BROWSE_FOR),
                look: 0.0,
            };
        }
        spot.is_some()
    }

    /// Caught in the rain, it steps under a free awning just ahead, if
    /// there is one, to wait for the rain to ease. Returns whether it does.
    pub(super) fn take_shelter(
        &mut self,
        places: &Places,
        seats: &mut Seats,
        rng: &mut impl Rng,
    ) -> bool {
        let spot = self.free_ahead(&places.awnings, &seats.awnings, false, |spot| {
            ((), spot.at.translation.xz())
        });
        if let Some((spot, ())) = spot {
            seats.awnings[spot] = true;
            self.activity = Activity::Shelter {
                spot,
                arrived: false,
                left: rng.random_range(SHELTER_FOR),
                look: 0.0,
                eased: 0.0,
            };
        }
        spot.is_some()
    }

    /// The nearest free one of `spots` just ahead on its sidewalk, out of
    /// the rain unless `dry` anyway, with what `approach` says about it,
    /// and where it goes to first.
    fn free_ahead<T>(
        &self,
        spots: &[Spot],
        taken: &[bool],
        dry: bool,
        approach: impl Fn(&Spot) -> (T, Vec2),
    ) -> Option<(usize, T)> {
        spots
            .iter()
            .enumerate()
            .filter(|&(index, spot)| {
                !taken[index] && self.reaches(spot.at.translation) && (dry || spot.covered)
            })
            .map(|(index, spot)| {
                let (how, to) = approach(spot);
                (index, how, (to.x - self.x) * self.heading)
            })
            .filter(|(.., ahead)| SEAT_AHEAD.contains(ahead))
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(index, how, _)| (index, how))
    }

    /// Does what it set out to do at the stalls: looks over the goods at a
    /// counter, or walks to the side of a stool and sits on it, stays,
    /// gets up and walks on.
    #[expect(clippy::too_many_arguments, reason = "all that an outing touches")]
    pub(super) fn go_out(
        &mut self,
        bird: &mut Bird,
        gaze: &mut Gaze,
        voice: &mut Voice,
        around: &Around,
        seats: &mut Seats,
        dt: f32,
        rng: &mut impl Rng,
    ) {
        let Around {
            places,
            customers,
            cars,
            raining,
        } = *around;
        let stools = &places.stools;
        match self.activity {
            Activity::Stroll | Activity::Visit => {}
            Activity::Cross { .. } => self.cross(bird, gaze, cars, dt, rng),
            Activity::Chat {
                with,
                mut left,
                heading,
                then,
            } => {
                let change = ACCELERATION * dt;
                bird.speed = (bird.speed - change).max(0.0);
                let way = with - Vec2::new(self.x, self.z);
                self.yaw = turned(self.yaw, way.x.atan2(way.y), TURN_RATE * 0.5 * dt);
                gaze.glance_at(self.turn_to(with), 0.5);
                voice.chat(CHAT_TALK);
                left -= dt;
                if left <= 0.0 {
                    voice.talk_every(WALKER_TALK);
                    match then {
                        Some((stool, side)) => self.activity = Activity::ToSeat { stool, side },
                        None => self.walk_on(heading, rng),
                    }
                } else {
                    self.activity = Activity::Chat {
                        with,
                        left,
                        heading,
                        then,
                    };
                }
            }
            Activity::Browse {
                spot: index,
                mut arrived,
                mut left,
                mut look,
            } => {
                let spot = &places.standing[index];
                if !arrived {
                    arrived = self.walk_to(bird, spot.at.translation.xz(), bird.pace, dt);
                } else {
                    bird.speed = 0.0;
                    self.yaw = turned(self.yaw, facing(spot), TURN_RATE * dt);
                    look -= dt;
                    if look <= 0.0 {
                        look = rng.random_range(LOOK_FOR);
                        let turn = Vec2::new(
                            rng.random_range(-LOOK_ASIDE..LOOK_ASIDE),
                            rng.random_range(LOOK_DOWN),
                        );
                        gaze.glance_at(turn, look);
                    }
                    left -= dt;
                }
                bird.lean_in(if arrived { LEAN_IN } else { 0.0 }, dt);
                if left <= 0.0 {
                    seats.standing[index] = false;
                    let heading = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                    self.walk_on(heading, rng);
                } else {
                    self.activity = Activity::Browse {
                        spot: index,
                        arrived,
                        left,
                        look,
                    };
                }
            }
            Activity::Shelter {
                spot: index,
                mut arrived,
                mut left,
                mut look,
                mut eased,
            } => {
                let spot = &places.awnings[index];
                if !arrived {
                    arrived = self.walk_to(bird, spot.at.translation.xz(), bird.pace, dt);
                } else {
                    bird.speed = 0.0;
                    self.yaw = turned(self.yaw, facing(spot), TURN_RATE * dt);
                    look -= dt;
                    if look <= 0.0 {
                        look = rng.random_range(SHELTER_LOOK);
                        // Up at the sky, or along the street.
                        let turn = if rng.random_bool(0.4) {
                            Vec2::new(rng.random_range(-0.3..0.3), -0.5)
                        } else {
                            let side = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                            Vec2::new(side * rng.random_range(0.6..1.2), 0.0)
                        };
                        gaze.glance_at(turn, look);
                    }
                    left -= dt;
                }
                bird.stand_under(arrived);
                eased = if raining { 0.0 } else { eased + dt };
                if left <= 0.0 || eased >= EASED_FOR {
                    if left <= 0.0 {
                        self.until_shelter = rng.random_range(SHELTER_AGAIN);
                    }
                    seats.awnings[index] = false;
                    bird.stand_under(false);
                    let heading = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                    self.walk_on(heading, rng);
                } else {
                    self.activity = Activity::Shelter {
                        spot: index,
                        arrived,
                        left,
                        look,
                        eased,
                    };
                }
            }
            Activity::ToSeat { stool: index, side } => {
                let stool = &stools[index];
                if self.walk_to(bird, beside(stool, side), bird.pace, dt) {
                    self.activity = Activity::Seated {
                        stool: index,
                        side,
                        sitting: 0.0,
                        turn: rng.random_range(-SEAT_TURN..SEAT_TURN),
                        dining: Dining::order(bird.has_umbrella_open(), rng),
                        left: rng.random_range(SIT_FOR),
                        rising: false,
                        glance: rng.random_range(TURN_EVERY),
                    };
                }
            }
            Activity::Seated {
                stool: index,
                side,
                mut sitting,
                turn,
                mut dining,
                mut left,
                mut rising,
                mut glance,
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
                    left -= dt;
                    dining.fit_umbrella(bird.has_umbrella_open());
                    bird.eat(dining.go_on(dt, left <= 0.0, rng));
                    rising = left <= 0.0 && dining.is_done();
                    glance -= dt;
                    if glance <= 0.0 {
                        glance = rng.random_range(TURN_EVERY);
                        let me = Vec2::new(self.x, self.z);
                        let neighbour = customers
                            .iter()
                            .map(|at| at.xz())
                            .filter(|at| (0.1..NEIGHBOUR).contains(&at.distance(me)))
                            .min_by(|a, b| a.distance(me).total_cmp(&b.distance(me)));
                        if let Some(at) = neighbour {
                            gaze.glance_at(self.turn_to(at), rng.random_range(TURN_FOR));
                            voice.chat(CHAT_TALK);
                        } else {
                            voice.talk_every(SITTER_TALK);
                        }
                    }
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
                    seats.stools[index] = false;
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
                        dining,
                        left,
                        rising,
                        glance,
                    };
                }
            }
        }
    }

    /// Steps towards `to` at `speed`, slowing down as it gets there.
    /// Returns whether it is there.
    pub(super) fn walk_to(&mut self, bird: &mut Bird, to: Vec2, speed: f32, dt: f32) -> bool {
        let way = to - Vec2::new(self.x, self.z);
        let distance = way.length();
        let target = speed * (distance / SLOW_DOWN).min(1.0);
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
fn beside(stool: &Spot, side: f32) -> Vec2 {
    stool.at.transform_point(Vec3::X * side * BESIDE).xz()
}

/// Which way a stool faces, as a yaw.
fn facing(stool: &Spot) -> f32 {
    let ahead = stool.at.rotation * Vec3::Z;
    ahead.x.atan2(ahead.z)
}

/// `from` turned towards `to` by up to `step`, the short way round.
pub(super) fn turned(from: f32, to: f32, step: f32) -> f32 {
    let difference = (to - from + PI).rem_euclid(TAU) - PI;
    from + difference.clamp(-step, step)
}

pub(super) fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stool(x: f32, covered: bool) -> Spot {
        Spot {
            at: Transform::from_xyz(x, 0.0, -2.0),
            height: 0.75,
            covered,
        }
    }

    #[test]
    fn friends_take_neighbouring_stools_without_crossing() {
        let stools = [stool(-6.0, true), stool(0.5, true), stool(-0.5, true)];
        let mut seats = Seats {
            stools: vec![false; 3],
            standing: Vec::new(),
            awnings: Vec::new(),
        };
        let at = [Vec2::new(1.0, -2.5), Vec2::new(-1.0, -2.5)];
        let taken = seats.side_by_side(&stools, at, true, |_| true);
        assert_eq!(taken, Some([(1, 1.0), (2, -1.0)]));
        assert_eq!(seats.stools, [false, true, true]);
    }

    #[test]
    fn friends_stay_dry_in_the_rain() {
        let stools = [stool(-0.5, true), stool(0.5, false)];
        let mut seats = Seats {
            stools: vec![false; 2],
            standing: Vec::new(),
            awnings: Vec::new(),
        };
        let at = [Vec2::new(-1.0, -2.5), Vec2::new(1.0, -2.5)];
        assert_eq!(seats.side_by_side(&stools, at, false, |_| true), None);
    }
}
