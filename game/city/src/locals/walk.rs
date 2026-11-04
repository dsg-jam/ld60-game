use std::f32::consts::FRAC_PI_2;
use std::iter;
use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::super::weather::Weather;
use super::bird::Bird;
use super::gaze::Gaze;
use super::{Places, Whims};

/// The sidewalks people walk along, each with a lane per direction.
const SIDEWALKS: [Sidewalk; 2] = [
    Sidewalk {
        lanes: [-2.5..-2.1, -3.1..-2.7],
        sheltered: Some([-1.45, -1.75]),
        room: -3.5..-0.95,
        walkers: 10,
    },
    Sidewalk {
        lanes: [-14.3..-14.0, -14.8..-14.5],
        sheltered: None,
        room: -15.3..-13.6,
        walkers: 12,
    },
];
/// People walk this far to each side of the shop before they come back
/// along the other end.
const WANDER: f32 = 45.0;
/// Meters per second squared.
const ACCELERATION: f32 = 1.2;
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
/// People only stop near the shop, where the stalls are lit.
const STOP_NEAR: f32 = 22.0;

struct Sidewalk {
    /// The range of Z for walking towards +X and towards -X. The lane
    /// towards +X has the larger Z.
    lanes: [Range<f32>; 2],
    /// Z under the awnings, if there are any.
    sheltered: Option<[f32; 2]>,
    /// The range of Z people may walk in.
    room: Range<f32>,
    walkers: usize,
}

/// Someone or something on a sidewalk, seen from above.
#[derive(Debug, Clone, Copy)]
pub(super) struct Body {
    at: Vec2,
    /// Speed along X.
    velocity: f32,
    radius: f32,
}

/// Walks a bird along a sidewalk, past the shop and back around.
#[derive(Component, Debug)]
pub(super) struct Walker {
    x: f32,
    z: f32,
    /// +1 or -1 along X.
    heading: f32,
    lane: f32,
    sheltered: Option<f32>,
    room: Range<f32>,
    /// The side of the street the stalls are on, along Z.
    stalls: f32,
    yaw: f32,
    until_stop: f32,
    stopped: f32,
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
            .enumerate()
            .flat_map(|(index, sidewalk)| {
                let spacing = 2.0 * WANDER / sidewalk.walkers as f32;
                (0..sidewalk.walkers).map(move |n| (index, sidewalk, -WANDER + spacing * n as f32))
            })
            .map(|(index, sidewalk, x)| {
                let direction = usize::from(rng.random_bool(0.5));
                let heading = if direction == 0 { 1.0 } else { -1.0 };
                let lane = rng.random_range(sidewalk.lanes[direction].clone());
                Self {
                    x: x + rng.random_range(0.0..2.0 * WANDER / sidewalk.walkers as f32 * 0.7),
                    z: lane,
                    heading,
                    lane,
                    sheltered: sidewalk.sheltered.map(|lanes| lanes[direction]),
                    room: sidewalk.room.clone(),
                    stalls: if index == 0 { 1.0 } else { -1.0 },
                    yaw: heading * FRAC_PI_2,
                    until_stop: rng.random_range(STOP_EVERY),
                    stopped: 0.0,
                }
            })
    }

    pub(super) fn place(&self) -> Transform {
        Transform::from_xyz(self.x, 0.0, self.z).with_rotation(Quat::from_rotation_y(self.yaw))
    }

    pub(super) fn walk(
        time: Res<Time>,
        weather: Res<Weather>,
        places: Res<Places>,
        mut whims: ResMut<Whims>,
        mut bodies: Local<Vec<Body>>,
        mut walkers: Query<(&mut Walker, &mut Bird, &mut Gaze, &mut Transform)>,
    ) {
        let dt = time.delta_secs();
        let rng = &mut whims.0;
        bodies.clear();
        bodies.extend(places.in_way.iter().map(|&at| Body {
            at,
            velocity: 0.0,
            radius: STOOL,
        }));
        let first = bodies.len();
        bodies.extend(walkers.iter().map(|(walker, bird, ..)| walker.body(bird)));
        for (index, (mut walker, mut bird, mut gaze, mut transform)) in
            walkers.iter_mut().enumerate()
        {
            let rained_on = bird.rained_on(weather.rain);
            bird.hunch += (rained_on - bird.hunch) * (1.0 - (-dt).exp());

            let mut target = bird.pace * (1.0 + HURRY * rained_on);
            if walker.stopped > 0.0 {
                walker.stopped -= dt;
                target = 0.0;
            } else if walker.until_stop <= 0.0 {
                walker.until_stop = rng.random_range(STOP_EVERY);
                if walker.x.abs() < STOP_NEAR && rained_on < SHELTER_RAIN {
                    walker.stopped = rng.random_range(STOP_FOR);
                    let side = -walker.stalls * walker.heading;
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

            let lane = match walker.sheltered {
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
            .chain([self.room.start, self.room.end])
            .chain(near.clone().flat_map(|other| {
                let room = me.radius + other.radius;
                [other.at.y - room, other.at.y + room]
            }))
            .map(|z| z.clamp(self.room.start, self.room.end))
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
