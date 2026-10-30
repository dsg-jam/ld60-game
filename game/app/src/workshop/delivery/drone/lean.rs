use bevy::prelude::*;
use ld60_game_core::GRAVITY;

/// How fast the drone's controller brings it to the lean it needs, as the
/// angular frequency of a critically damped spring.
const STIFFNESS: f32 = 9.0;
/// Air drag on the drone, in meters per second squared per meter per
/// second. It leans a little into the air while it flies at a steady speed.
const DRAG: f32 = 0.6;
/// It never leans further than this, in radians.
const MOST: f32 = 0.45;
/// The small corrections of a hovering drone: how far, in radians, and how
/// often, in hertz.
const WOBBLE: f32 = 0.012;
const WOBBLE_RATES: [f32; 3] = [0.7, 1.3, 2.1];
/// Longer steps than this, in seconds, are split up.
const MAX_STEP: f32 = 1.0 / 60.0;

/// How a quadcopter leans. It can only push along its own up, so to speed
/// up, slow down or hold against the air it tilts its thrust that way.
#[derive(Debug, Default)]
pub(super) struct Lean {
    /// As a scaled axis.
    angle: Vec3,
    speed: Vec3,
    /// Seconds of flying, for the wobble.
    time: f32,
    /// A different wobble for each drone.
    phase: f32,
}

impl Lean {
    pub(super) fn new(phase: f32) -> Self {
        Self { phase, ..default() }
    }

    pub(super) fn rotation(&self) -> Quat {
        let wobble = |index: usize| {
            let rate = WOBBLE_RATES[index];
            (std::f32::consts::TAU * rate * self.time + self.phase * (index + 1) as f32).sin()
        };
        let wobble = Vec3::new(wobble(0) + wobble(2), 0.0, wobble(1) + wobble(2)) * WOBBLE / 2.0;
        Quat::from_scaled_axis(self.angle + wobble)
    }

    /// Leans on for `dt` seconds while the drone flies at `velocity` and
    /// speeds up by `acceleration`.
    pub(super) fn step(&mut self, acceleration: Vec3, velocity: Vec3, dt: f32) {
        self.time += dt;
        let push = (acceleration + DRAG * velocity).with_y(0.0);
        let thrust = push + Vec3::Y * (GRAVITY + acceleration.y).max(GRAVITY / 2.0);
        let wanted = Quat::from_rotation_arc(Vec3::Y, thrust.normalize())
            .to_scaled_axis()
            .clamp_length_max(MOST);
        let mut left = dt;
        while left > 0.0 {
            let step = left.min(MAX_STEP);
            let pull = STIFFNESS * STIFFNESS * (wanted - self.angle) - 2.0 * STIFFNESS * self.speed;
            self.speed += pull * step;
            self.angle += self.speed * step;
            left -= step;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tilt(lean: &Lean) -> Vec3 {
        Quat::from_scaled_axis(lean.angle) * Vec3::Y
    }

    #[test]
    fn it_leans_into_a_push_slowly_and_levels_out_after() {
        let mut lean = Lean::default();
        let dt = 1.0 / 60.0;
        lean.step(Vec3::X * 3.0, Vec3::ZERO, dt);
        assert!(tilt(&lean).x < 0.01, "{}", tilt(&lean));
        for _ in 0..60 {
            lean.step(Vec3::X * 3.0, Vec3::ZERO, dt);
        }
        let leaned = tilt(&lean);
        assert!(leaned.x > 0.25 && leaned.x < 0.35, "{leaned}");
        for _ in 0..120 {
            lean.step(Vec3::ZERO, Vec3::ZERO, dt);
        }
        assert!(tilt(&lean).x.abs() < 1e-3, "{}", tilt(&lean));
    }

    #[test]
    fn it_leans_into_the_air_at_a_steady_speed() {
        let mut lean = Lean::default();
        for _ in 0..120 {
            lean.step(Vec3::ZERO, Vec3::NEG_Z, 1.0 / 60.0);
        }
        assert!(tilt(&lean).z < -0.02, "{}", tilt(&lean));
    }
}
