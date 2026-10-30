use std::f32::consts::TAU;

use bevy::{platform::hash::fixed_hash_one, prelude::*};
use ld60_game_city::Draft;
use ld60_game_parts::paper::Swing;
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
/// Radians of flutter per meter per second of draft.
const FLUTTER: f32 = 0.015;
const FLUTTER_RATE: f32 = 2.5;

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
    /// About the top edge. Positive swings the bottom back.
    swing: Swing,
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
            swing: Swing::new(SWING_RATE, DAMPING, PUSH),
            flutter: slot as f32 * 0.37,
            slot,
        }
    }

    /// The pose of the sheet in the rail's space, swung by `angle` about
    /// the top edge.
    fn pose(&self, angle: f32) -> Transform {
        let rotation = Quat::from_rotation_z(self.tilt) * Quat::from_rotation_x(angle);
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
            let dt = time.delta_secs();
            sway.swing.step(against, dt);
            sway.flutter += dt;
            let gust = sway.swing.gust().abs();
            let flutter = (TAU * FLUTTER_RATE * sway.flutter).sin() * gust * FLUTTER;
            let pose = sway.pose(sway.swing.angle() + flutter);
            if *transform != pose {
                *transform = pose;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sway() -> Sway {
        Sway::new(Vec3::ZERO, Vec3::NEG_Y * 0.07, 0)
    }

    #[test]
    fn a_gust_swings_the_bottom_back() {
        let mut sway = sway();
        for _ in 0..30 {
            sway.swing.step(0.2, 1.0 / 60.0);
        }
        let bottom = sway
            .pose(sway.swing.angle())
            .transform_point(Vec3::NEG_Y * 0.07);
        assert!(bottom.z < -0.001, "{bottom}");
    }

    #[test]
    fn the_clip_stays_put() {
        let mut sway = sway();
        sway.tilt = TILT;
        let pose = sway.pose(0.25);
        let clip = pose.transform_point(Vec3::NEG_Y * 0.07 * -1.0);
        assert!(clip.length() < 1e-5, "{clip}");
    }
}
