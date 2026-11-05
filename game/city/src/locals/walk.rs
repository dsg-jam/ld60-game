mod meal;
mod outing;

use std::f32::consts::{FRAC_PI_2, TAU};
use std::iter;
use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use self::outing::Activity;
pub(super) use self::outing::{Customers, Seats};
use super::super::weather::Weather;
use super::bird::Bird;
use super::gaze::Gaze;
use super::sound::Voice;
use super::{Places, Whims};

/// The sidewalks people walk along, each with a lane per direction.
static SIDEWALKS: [Sidewalk; 2] = [
    Sidewalk {
        lanes: [-2.6..-1.9, -3.3..-2.6],
        sheltered: Some([-1.45, -1.75]),
        room: -3.5..-0.95,
        walkers: 10,
        stalls: 1.0,
    },
    Sidewalk {
        lanes: [-14.5..-13.8, -15.2..-14.5],
        sheltered: None,
        room: -15.3..-13.6,
        walkers: 12,
        stalls: -1.0,
    },
];
/// People walk this far to each side of the shop before they come back
/// along the other end.
const WANDER: f32 = 45.0;
/// Meters per second squared.
pub(super) const ACCELERATION: f32 = 1.2;
/// How fast people step aside, in meters per second.
const SIDESTEP: f32 = 0.6;
/// People hurry this much more when the rain falls on them.
const HURRY: f32 = 0.35;
/// The rain is hard enough to look for shelter.
const SHELTER_RAIN: f32 = 0.3;
/// People keep this much room to someone slower ahead of them.
const GAP: f32 = 0.5;
/// Squeezing past someone costs as much as stepping this far aside, per
/// meter of overlap.
const SQUEEZE: f32 = 8.0;
/// Standing, people step aside this much slower than walking.
const SHUFFLE: f32 = 0.5;
/// Seconds ahead people look for someone in their way.
const FORESIGHT: f32 = 2.5;
/// Passing someone coming the other way on the wrong side costs as much
/// as stepping this far aside.
const WRONG_SIDE: f32 = 1.0;
/// Umbrellas tilt away from others closer than this.
const CROWDED: f32 = 1.1;
/// Room taken by a stool, with or without someone on it.
const STOOL: f32 = 0.32;
/// How far people walk between stops, and how long they stand.
const STOP_EVERY: Range<f32> = 15.0..60.0;
const STOP_FOR: Range<f32> = 2.0..6.0;
/// People mostly stop near the shop, where the stalls are lit. Elsewhere
/// only this share of the stops happen, for a look around.
const STOP_NEAR: f32 = 22.0;
const STOP_AWAY: f64 = 0.3;
/// Seconds between picking a new place across the sidewalk to walk at,
/// and how fast people drift over to it.
const DRIFT_EVERY: Range<f32> = 4.0..14.0;
const DRIFT: f32 = 0.12;
/// People walk a little faster and slower as they go, this much, over
/// about this many seconds.
const WOBBLE: f32 = 0.08;
const WOBBLE_TIME: Range<f32> = 6.0..14.0;

#[derive(Debug)]
struct Sidewalk {
    /// The range of Z for walking towards +X and towards -X. The lane
    /// towards +X has the larger Z.
    lanes: [Range<f32>; 2],
    /// Z under the awnings, if there are any.
    sheltered: Option<[f32; 2]>,
    /// The range of Z people may walk in.
    room: Range<f32>,
    walkers: usize,
    /// The side of the street the stalls are on, along Z.
    stalls: f32,
}

/// Someone or something on a sidewalk, seen from above.
#[derive(Debug, Clone, Copy)]
pub(super) struct Body {
    at: Vec2,
    /// Speed along X.
    velocity: f32,
    radius: f32,
}

/// Takes a bird out along a sidewalk, past the shop and back around. On
/// the way it stops to look at the stalls, and sits down on their stools
/// for a while.
#[derive(Component, Debug)]
pub(super) struct Walker {
    sidewalk: &'static Sidewalk,
    x: f32,
    z: f32,
    /// +1 or -1 along X.
    heading: f32,
    /// The Z it walks at, drifting towards `wish`, which it picks anew
    /// within its lane now and then.
    lane: f32,
    wish: f32,
    until_drift: f32,
    /// How far through its slow changes of speed, and how fast they go.
    wobble: f32,
    wobble_rate: f32,
    yaw: f32,
    until_stop: f32,
    stopped: f32,
    /// Meters left until it looks for a seat.
    until_seat: f32,
    activity: Activity,
}

impl Body {
    /// The way away from the nearest of `others` that crowds it, and how
    /// close it is, from 0 to 1.
    fn crowded_by<'a>(&self, others: impl Iterator<Item = &'a Body>) -> Option<(Vec2, f32)> {
        others
            .map(|other| (self.at - other.at, self.at.distance(other.at)))
            .filter(|&(_, distance)| distance < CROWDED)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(away, distance)| (away, 1.0 - distance / CROWDED))
    }
}

impl Walker {
    /// Walkers spread out along every sidewalk.
    pub(super) fn crowd(rng: &mut impl Rng) -> impl Iterator<Item = Self> {
        SIDEWALKS
            .iter()
            .flat_map(|sidewalk| {
                let spacing = 2.0 * WANDER / sidewalk.walkers as f32;
                (0..sidewalk.walkers).map(move |n| (sidewalk, -WANDER + spacing * n as f32))
            })
            .map(|(sidewalk, x)| {
                let mut walker = Self::on(sidewalk, rng);
                walker.x = x + rng.random_range(0.0..2.0 * WANDER / sidewalk.walkers as f32 * 0.7);
                walker
            })
    }

    /// A walker for a stool at `at` on whichever sidewalk reaches it.
    pub(super) fn near(at: Vec3, rng: &mut impl Rng) -> Option<Self> {
        SIDEWALKS
            .iter()
            .map(|sidewalk| Self::on(sidewalk, rng))
            .find(|walker| walker.reaches(at))
    }

    /// A walker on `sidewalk`, going either way.
    fn on(sidewalk: &'static Sidewalk, rng: &mut impl Rng) -> Self {
        let heading = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
        let mut walker = Self {
            sidewalk,
            x: 0.0,
            z: 0.0,
            heading,
            lane: 0.0,
            wish: 0.0,
            until_drift: rng.random_range(DRIFT_EVERY),
            wobble: rng.random_range(0.0..TAU),
            wobble_rate: TAU / rng.random_range(WOBBLE_TIME),
            yaw: heading * FRAC_PI_2,
            until_stop: rng.random_range(STOP_EVERY),
            stopped: 0.0,
            until_seat: 0.0,
            activity: Activity::Stroll,
        };
        walker.walk_on(heading, rng);
        walker.z = walker.lane;
        walker
    }

    /// Its own side of the sidewalk, for the way it goes.
    fn lanes(&self) -> Range<f32> {
        self.sidewalk.lanes[usize::from(self.heading < 0.0)].clone()
    }

    /// Where it walks out of the rain, if anywhere.
    fn shelter(&self) -> Option<f32> {
        self.sidewalk
            .sheltered
            .map(|lanes| lanes[usize::from(self.heading < 0.0)])
    }

    pub(super) fn place(&self) -> Transform {
        Transform::from_xyz(self.x, 0.0, self.z).with_rotation(Quat::from_rotation_y(self.yaw))
    }

    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    pub(super) fn walk(
        time: Res<Time>,
        weather: Res<Weather>,
        places: Res<Places>,
        mut seats: ResMut<Seats>,
        mut customers: ResMut<Customers>,
        mut whims: ResMut<Whims>,
        mut bodies: Local<Vec<Body>>,
        mut walkers: Query<(
            &mut Walker,
            &mut Bird,
            &mut Gaze,
            &mut Voice,
            &mut Transform,
        )>,
    ) {
        let dt = time.delta_secs();
        let rng = &mut whims.0;
        customers.0.clear();
        bodies.clear();
        bodies.extend(places.in_way.iter().map(|&at| Body {
            at,
            velocity: 0.0,
            radius: STOOL,
        }));
        let first = bodies.len();
        bodies.extend(walkers.iter().map(|(walker, bird, ..)| walker.body(bird)));
        for (index, (mut walker, mut bird, mut gaze, mut voice, mut transform)) in
            walkers.iter_mut().enumerate()
        {
            let rained_on = bird.rained_on(weather.rain);
            bird.hunch += (rained_on - bird.hunch) * (1.0 - (-dt).exp());
            if !walker.activity.is_stroll() {
                walker.go_out(
                    &mut bird, &mut gaze, &mut voice, &places, &mut seats, dt, rng,
                );
                if walker.activity.is_customer() {
                    customers.0.push(transform.translation);
                }
                *transform = walker.place();
                continue;
            }
            walker.look_for_seat(
                &bird,
                &places,
                &mut seats,
                weather.rain >= SHELTER_RAIN,
                dt,
                rng,
            );

            walker.drift(dt, rng);
            let mut target =
                bird.pace * (1.0 + HURRY * rained_on) * (1.0 + WOBBLE * walker.wobble.sin());
            if walker.stopped > 0.0 {
                walker.stopped -= dt;
                target = 0.0;
            } else if walker.until_stop <= 0.0 {
                walker.until_stop = rng.random_range(STOP_EVERY);
                let raining = weather.rain >= SHELTER_RAIN;
                let here = walker.x.abs() < STOP_NEAR || rng.random_bool(STOP_AWAY);
                if walker.browse(&bird, &places, &mut seats, raining, rng) {
                    target = 0.0;
                } else if here && rained_on < SHELTER_RAIN {
                    walker.stopped = rng.random_range(STOP_FOR);
                    let side = -walker.sidewalk.stalls * walker.heading;
                    gaze.glance_at(Vec2::new(side * 1.2, 0.1), walker.stopped);
                }
            }
            let me = walker.body(&bird);
            let others = bodies
                .iter()
                .enumerate()
                .filter(|&(other, _)| other != first + index)
                .map(|(_, body)| body);
            if let Some((gap, speed)) = walker.blocked(&me, others.clone()) {
                let slower = speed.min(target);
                target = if gap < GAP {
                    slower * gap.max(0.0) / GAP
                } else {
                    slower + (target - slower) * ((gap - GAP) / GAP).min(1.0)
                };
            }
            let change = ACCELERATION * dt;
            bird.speed += (target - bird.speed).clamp(-change, change);
            walker.until_stop -= bird.speed * dt;

            let lane = match walker.shelter() {
                Some(z) if rained_on > SHELTER_RAIN => z,
                _ => walker.lane,
            };
            let lane = walker.steer(lane, &me, others.clone());
            if let Some((away, crowded)) = me.crowded_by(others) {
                let away = Quat::from_rotation_y(-walker.yaw) * Vec3::new(away.x, 0.0, away.y);
                bird.lean_umbrella(away.xz(), crowded, dt);
            }
            // Standing people still shuffle aside.
            let step = SIDESTEP * dt * bird.speed.clamp(SHUFFLE, 1.0);
            let dz = (lane - walker.z).clamp(-step, step);
            walker.z += dz;
            walker.x += walker.heading * bird.speed * dt;
            if walker.x.abs() > WANDER {
                walker.x -= walker.heading * 2.0 * WANDER;
            }
            if bird.speed > 0.05 {
                let yaw = (walker.heading * bird.speed * dt).atan2(dz);
                walker.yaw += (yaw - walker.yaw) * (1.0 - (-4.0 * dt).exp());
            }
            *transform = walker.place();
        }
    }

    /// Drifts across its side of the sidewalk now and then, and walks a
    /// little faster or slower, so no two walk in step for long.
    fn drift(&mut self, dt: f32, rng: &mut impl Rng) {
        self.wobble = (self.wobble + self.wobble_rate * dt) % TAU;
        self.until_drift -= dt;
        if self.until_drift <= 0.0 {
            self.until_drift = rng.random_range(DRIFT_EVERY);
            self.wish = rng.random_range(self.lanes());
        }
        let step = DRIFT * dt;
        self.lane += (self.wish - self.lane).clamp(-step, step);
    }

    fn body(&self, bird: &Bird) -> Body {
        Body {
            at: Vec2::new(self.x, self.z),
            velocity: self.heading * bird.speed,
            radius: bird.radius(),
        }
    }

    /// How far ahead `other` is, if it will be in the way soon.
    fn ahead_of(&self, me: &Body, other: &Body) -> Option<f32> {
        let ahead = (other.at.x - me.at.x) * self.heading;
        let room = me.radius + other.radius;
        let closing = (me.velocity - other.velocity) * self.heading;
        (ahead > -room * 0.5 && ahead < room + 0.3 + closing.max(0.0) * FORESIGHT).then_some(ahead)
    }

    /// The Z to walk at instead of `lane`, to get around everyone in the
    /// way. People coming the other way are passed on the side of the own
    /// lane.
    fn steer<'a>(
        &self,
        lane: f32,
        me: &Body,
        others: impl Iterator<Item = &'a Body> + Clone,
    ) -> f32 {
        let near = others.filter(|other| self.ahead_of(me, other).is_some());
        // In a crowd no spot may be clear. Then the least squeezed one
        // wins, so nobody waits for a gap that never comes.
        let cost = |z: f32| {
            let (wrong, squeeze) = near.clone().fold((0, 0.0), |(wrong, squeeze), other| {
                let oncoming = other.velocity * self.heading < 0.0;
                let wrong_side = oncoming && (z - other.at.y) * self.heading < 0.0;
                let overlap = (me.radius + other.radius - (z - other.at.y).abs()).max(0.0);
                (wrong + usize::from(wrong_side), squeeze + overlap)
            });
            (z - lane).abs() + WRONG_SIDE * wrong as f32 + SQUEEZE * squeeze
        };
        iter::once(lane)
            .chain([self.sidewalk.room.start, self.sidewalk.room.end])
            .chain(near.clone().flat_map(|other| {
                let room = me.radius + other.radius;
                [other.at.y - room, other.at.y + room]
            }))
            .map(|z| z.clamp(self.sidewalk.room.start, self.sidewalk.room.end))
            .min_by(|&a, &b| cost(a).total_cmp(&cost(b)))
            .unwrap_or(lane)
    }

    /// The room to the nearest one ahead in the way, and its speed in the
    /// same direction.
    fn blocked<'a>(&self, me: &Body, others: impl Iterator<Item = &'a Body>) -> Option<(f32, f32)> {
        others
            .filter(|other| (other.at.y - me.at.y).abs() < (me.radius + other.radius) * 0.9)
            .filter_map(|other| {
                let ahead = (other.at.x - me.at.x) * self.heading;
                let room = ahead - me.radius - other.radius;
                (ahead > 0.01 && room < 2.0 * GAP)
                    .then(|| (room, (other.velocity * self.heading).max(0.0)))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
    }
}
