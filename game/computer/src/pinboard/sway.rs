use std::f32::consts::TAU;

use bevy::{platform::hash::fixed_hash_one, prelude::*};
use ld60_game_city::Draft;
use rand::{RngExt, SeedableRng, rngs::SmallRng};

use super::Hanging;

/// The most a sheet tilts about its clip, as if clipped on in a rush, in
/// radians.
const TILT: f32 = 0.045;
/// The least it tilts, so each sheet looks a little off.
const MIN_TILT: f32 = 0.012;
/// How often a sheet swings back and forth on its own, in hertz. Slower
/// than a stiff plate, because the air moves with the paper.
const SWING_RATE: f32 = 0.8;
/// The share of the critical damping, which the paper's drag in the air
/// gives the swing.
const DAMPING: f32 = 0.35;
/// Angular acceleration per meter per second of draft against the sheet.
const PUSH: f32 = 9.0;
/// Seconds for the draft on a sheet to build up and die down.
const GUST_TIME: f32 = 0.35;
/// Radians of flutter per meter per second of draft.
const FLUTTER: f32 = 0.015;
const FLUTTER_RATE: f32 = 2.5;
/// Longer steps than this, in seconds, are split up, so the swing stays
/// stable after a hitch.
const MAX_STEP: f32 = 1.0 / 60.0;

/// How a pinned sheet hangs from its clip: tilted a little and swinging in
/// the draft of passing vehicles. The edge in the clip stays put.
#[derive(Component, Debug)]
pub(crate) struct Sway {
    /// The middle of the top edge, in the rail's space.
    clip: Vec3,
    /// The middle of the sheet while it hangs straight.
    center: Vec3,
    /// About the sheet's normal, in radians.
    tilt: f32,
    /// About the top edge, in radians. Positive swings the bottom back.
    angle: f32,
    speed: f32,
    /// The draft against the sheet, smoothed, in meters per second.
    gust: f32,
    /// Seconds of flutter so far, offset for each sheet.
    flutter: f32,
    slot: usize,
}

impl Sway {
    pub(super) fn new(clip: Vec3, center: Vec3, slot: usize) -> Self {
        Self {
            clip,
            center,
            tilt: 0.0,
            angle: 0.0,
            speed: 0.0,
            gust: 0.0,
            flutter: slot as f32 * 0.37,
            slot,
        }
    }

    /// The pose of the sheet in the rail's space.
    fn pose(&self, flutter: f32) -> Transform {
        let rotation =
            Quat::from_rotation_z(self.tilt) * Quat::from_rotation_x(self.angle + flutter);
        Transform::from_translation(self.clip + rotation * (self.center - self.clip))
            .with_rotation(rotation)
    }

    /// Each page clipped on gets a tilt of its own, which it keeps while
    /// it hangs.
    pub(crate) fn tilt_new_pages(mut sheets: Query<(&Hanging, &mut Sway), Changed<Hanging>>) {
        for (hanging, mut sway) in &mut sheets {
            let mut rng = SmallRng::seed_from_u64(fixed_hash_one(("tilt", sway.slot, hanging.0)));
            let tilt = rng.random_range(MIN_TILT..TILT);
            sway.tilt = if rng.random_bool(0.5) { tilt } else { -tilt };
        }
    }

    pub(crate) fn swing(
        time: Res<Time>,
        draft: Option<Res<Draft>>,
        mut sheets: Query<(&mut Sway, &mut Transform, &GlobalTransform)>,
    ) {
        let draft = draft.map_or(Vec3::ZERO, |draft| draft.0);
        for (mut sway, mut transform, global) in &mut sheets {
            let against = draft.dot(global.forward().as_vec3());
            sway.step(against, time.delta_secs());
            let flutter = (TAU * FLUTTER_RATE * sway.flutter).sin() * sway.gust.abs() * FLUTTER;
            let pose = sway.pose(flutter);
            if *transform != pose {
                *transform = pose;
            }
        }
    }

    /// Swings on by `dt` with `against` meters per second of draft blowing
    /// into the front of the sheet.
    fn step(&mut self, against: f32, dt: f32) {
        let omega = TAU * SWING_RATE;
        let mut left = dt;
        while left > 0.0 {
            let step = left.min(MAX_STEP);
            self.gust += (against - self.gust) * (step / GUST_TIME).min(1.0);
            let pull = -omega * omega * self.angle - 2.0 * DAMPING * omega * self.speed;
            self.speed += (pull + PUSH * self.gust) * step;
            self.angle += self.speed * step;
            left -= step;
        }
        self.flutter += dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sway() -> Sway {
        Sway::new(Vec3::ZERO, Vec3::NEG_Y * 0.07, 0)
    }

    #[test]
    fn a_gust_swings_the_sheet_back_and_it_settles() {
        let mut sway = sway();
        for _ in 0..30 {
            sway.step(0.2, 1.0 / 60.0);
        }
        let bottom = sway.pose(0.0).transform_point(Vec3::NEG_Y * 0.07);
        assert!(bottom.z < -0.001, "{bottom}");
        assert!(sway.angle.abs() < 0.1);
        for _ in 0..600 {
            sway.step(0.0, 1.0 / 60.0);
        }
        assert!(sway.angle.abs() < 1e-3);
    }

    #[test]
    fn the_clip_stays_put() {
        let mut sway = sway();
        sway.tilt = TILT;
        sway.angle = 0.2;
        let pose = sway.pose(0.05);
        let clip = pose.transform_point(Vec3::NEG_Y * 0.07 * -1.0);
        assert!(clip.length() < 1e-5, "{clip}");
    }

    #[test]
    fn long_frames_stay_stable() {
        let mut sway = sway();
        sway.step(1.0, 0.5);
        for _ in 0..20 {
            sway.step(0.0, 0.25);
        }
        assert!(sway.angle.abs() < 0.05);
    }
}
