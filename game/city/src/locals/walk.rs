use std::f32::consts::FRAC_PI_2;
use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::super::weather::Weather;
use super::Whims;
use super::gaze::Gaze;
use super::strider::Strider;

/// The sidewalks people walk along, each with a lane per direction.
const SIDEWALKS: [Sidewalk; 2] = [
    Sidewalk {
        lanes: [-2.5..-2.1, -3.1..-2.7],
        sheltered: Some([-1.45, -1.75]),
        walkers: 10,
    },
    Sidewalk {
        lanes: [-14.3..-14.0, -14.8..-14.5],
        sheltered: None,
        walkers: 12,
    },
];
/// People walk this far to each side of the shop before they come back
/// along the other end.
const WANDER: f32 = 45.0;
/// Meters per second squared.
const ACCELERATION: f32 = 1.2;
/// How fast people change lanes, in meters per second.
const SIDESTEP: f32 = 0.4;
/// People hurry this much more when the rain falls on them.
const HURRY: f32 = 0.35;
/// The rain is hard enough to look for shelter.
const SHELTER_RAIN: f32 = 0.3;
/// People keep this far behind someone slower ahead of them.
const GAP: f32 = 1.2;
/// People pass someone slower this far to the side, along +Z out in the
/// open, and take this many seconds for it.
const PASS: f32 = 0.6;
const PASS_TIME: f32 = 4.0;
/// How far people walk between stops, and how long they stand.
const STOP_EVERY: Range<f32> = 15.0..60.0;
const STOP_FOR: Range<f32> = 2.0..6.0;
/// People only stop near the shop, where the stalls are lit.
const STOP_NEAR: f32 = 22.0;

struct Sidewalk {
    /// The range of Z for walking towards +X and towards -X.
    lanes: [Range<f32>; 2],
    /// Z under the awnings, if there are any.
    sheltered: Option<[f32; 2]>,
    walkers: usize,
}

/// Walks a strider along a sidewalk, past the shop and back around.
#[derive(Component, Debug)]
pub(super) struct Walker {
    x: f32,
    z: f32,
    /// +1 or -1 along X.
    heading: f32,
    lane: f32,
    sheltered: Option<f32>,
    /// The side of the street the stalls are on, along Z.
    stalls: f32,
    yaw: f32,
    until_stop: f32,
    stopped: f32,
    /// Seconds left walking aside to pass someone.
    passing: f32,
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
                    stalls: if index == 0 { 1.0 } else { -1.0 },
                    yaw: heading * FRAC_PI_2,
                    until_stop: rng.random_range(STOP_EVERY),
                    stopped: 0.0,
                    passing: 0.0,
                }
            })
    }

    pub(super) fn place(&self) -> Transform {
        Transform::from_xyz(self.x, 0.0, self.z).with_rotation(Quat::from_rotation_y(self.yaw))
    }

    pub(super) fn walk(
        time: Res<Time>,
        weather: Res<Weather>,
        mut whims: ResMut<Whims>,
        mut ahead: Local<Vec<(f32, f32, f32, f32)>>,
        mut walkers: Query<(&mut Walker, &mut Strider, &mut Gaze, &mut Transform)>,
    ) {
        let dt = time.delta_secs();
        let rng = &mut whims.0;
        ahead.clear();
        ahead.extend(
            walkers
                .iter()
                .map(|(walker, strider, ..)| (walker.x, walker.z, walker.heading, strider.speed)),
        );
        for (mut walker, mut strider, mut gaze, mut transform) in &mut walkers {
            let covered = strider.has_umbrella_open();
            let rained_on = if covered { 0.0 } else { weather.rain };
            strider.hunch += (rained_on - strider.hunch) * (1.0 - (-dt).exp());

            let mut target = strider.pace * (1.0 + HURRY * rained_on);
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
            walker.passing -= dt;
            if let Some(gap) = walker.gap_ahead(&ahead) {
                if gap.1 < target * 0.9 && walker.stopped <= 0.0 {
                    walker.passing = PASS_TIME;
                }
                let (gap, slower) = (gap.0, gap.1.min(target));
                target = if gap < GAP {
                    slower * gap / GAP
                } else {
                    slower + (target - slower) * ((gap - GAP) / GAP).min(1.0)
                };
            }
            let change = ACCELERATION * dt;
            strider.speed += (target - strider.speed).clamp(-change, change);
            walker.until_stop -= strider.speed * dt;

            // Under the awnings, the stools are in the way along +Z.
            let (lane, pass) = match walker.sheltered {
                Some(z) if rained_on > SHELTER_RAIN => (z, -PASS),
                _ => (walker.lane, PASS),
            };
            let lane = if walker.passing > 0.0 {
                lane + pass
            } else {
                lane
            };
            let step = SIDESTEP * dt * strider.speed.min(1.0);
            let dz = (lane - walker.z).clamp(-step, step);
            walker.z += dz;
            walker.x += walker.heading * strider.speed * dt;
            if walker.x.abs() > WANDER {
                walker.x -= walker.heading * 2.0 * WANDER;
            }
            if strider.speed > 0.05 {
                let yaw = (walker.heading * strider.speed * dt).atan2(dz);
                walker.yaw += (yaw - walker.yaw) * (1.0 - (-4.0 * dt).exp());
            }
            *transform = walker.place();
        }
    }

    /// The distance to the nearest walker ahead in the same lane and its
    /// speed.
    fn gap_ahead(&self, others: &[(f32, f32, f32, f32)]) -> Option<(f32, f32)> {
        others
            .iter()
            .filter(|&&(_, z, heading, _)| heading == self.heading && (z - self.z).abs() < 0.35)
            .map(|&(x, _, _, speed)| ((x - self.x) * self.heading, speed))
            .filter(|&(gap, _)| gap > 0.01 && gap < 2.0 * GAP)
            .min_by(|a, b| a.0.total_cmp(&b.0))
    }
}
