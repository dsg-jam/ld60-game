use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use ld60_game_core::audio::{PlayAt, Sound};
use ld60_game_manuals::{MoveSheet, Pins};

/// Seconds for the rod to swing all the way out or back.
const SWING_TIME: f32 = 0.4;
/// The rod folded back along the side of the monitor, in radians.
const FOLDED: f32 = -FRAC_PI_2;
/// How long a page asked to be pinned may take to arrive, in seconds. The
/// rod stays out that long even if the page doesn't come.
const PIN_WAIT: f32 = 3.0;
/// How long the rail stays out once empty, in seconds, so an unpinned page
/// is well away before the rod folds.
const FOLD_DELAY: f32 = 0.25;

/// The rod of the pin rail on its hinge. It folds back along the side of
/// the monitor while nothing hangs from it, and swings out before a page
/// arrives.
#[derive(Component, Debug)]
pub(in crate::workshop::computer) struct Arm {
    /// The angle when out, in radians.
    swing: f32,
    /// From 0 folded to 1 out.
    out: f32,
    /// Seconds since a page was last asked to move to or from the rail.
    since_move: f32,
    /// Seconds since a page was asked to be pinned.
    since_pin: f32,
    /// Seconds the rail has been empty.
    empty_for: f32,
    /// Whether the rod has found its first pose.
    placed: bool,
}

impl Arm {
    pub(super) fn new(swing: f32) -> Self {
        Self {
            swing,
            out: 0.0,
            since_move: f32::INFINITY,
            since_pin: f32::INFINITY,
            empty_for: f32::INFINITY,
            placed: false,
        }
    }

    /// The rod's rotation about its hinge.
    pub(super) fn rotation(&self) -> Quat {
        let eased = self.out * self.out * (3.0 - 2.0 * self.out);
        Quat::from_rotation_y(FOLDED + (self.swing - FOLDED) * eased)
    }

    /// Starts swinging out as soon as a page is on its way.
    pub(in crate::workshop::computer) fn expect_pages(
        moved: On<MoveSheet>,
        mut arms: Query<&mut Arm>,
    ) {
        for mut arm in &mut arms {
            arm.since_move = 0.0;
            if matches!(*moved, MoveSheet::Pin(_)) {
                arm.since_pin = 0.0;
            }
        }
    }

    pub(in crate::workshop::computer) fn swing(
        time: Res<Time>,
        pins: Res<Pins>,
        mut arms: Query<(&mut Arm, &mut Transform, &GlobalTransform)>,
        mut commands: Commands,
    ) {
        let dt = time.delta_secs();
        let empty = pins.is_empty();
        for (mut arm, mut transform, hinge) in &mut arms {
            let before = arm.out;
            arm.step(empty, dt);
            if arm.out != before && (before == 0.0 || before == 1.0) {
                commands.trigger(PlayAt(Sound::Hinge, hinge.translation()));
            }
            let rotation = arm.rotation();
            if transform.rotation != rotation {
                transform.rotation = rotation;
            }
        }
    }

    /// Moves the rod on by `dt` with the rail `empty` or not. Pins that come
    /// or go without a page moving, like on loading a game, put the rod in
    /// place at once.
    fn step(&mut self, empty: bool, dt: f32) {
        self.since_move += dt;
        self.since_pin += dt;
        self.empty_for = if empty { self.empty_for + dt } else { 0.0 };
        let out = !empty || self.since_pin < PIN_WAIT || self.empty_for < FOLD_DELAY;
        let target = if out { 1.0 } else { 0.0 };
        if !self.placed || self.since_move > PIN_WAIT {
            self.out = if empty { 0.0 } else { 1.0 };
            self.placed = true;
        } else {
            let reach = dt / SWING_TIME;
            self.out += (target - self.out).clamp(-reach, reach);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn run(arm: &mut Arm, empty: bool, seconds: f32) {
        let mut elapsed = 0.0;
        while elapsed < seconds {
            arm.step(empty, DT);
            elapsed += DT;
        }
    }

    #[test]
    fn a_loaded_rail_starts_in_place() {
        let mut arm = Arm::new(0.5);
        arm.step(false, DT);
        assert_eq!(arm.out, 1.0);
        let mut arm = Arm::new(0.5);
        arm.step(true, DT);
        assert_eq!(arm.out, 0.0);
    }

    #[test]
    fn swings_out_before_the_page_arrives_and_back_after_it_left() {
        let mut arm = Arm::new(0.5);
        run(&mut arm, true, 1.0);
        arm.since_pin = 0.0;
        arm.since_move = 0.0;
        run(&mut arm, true, SWING_TIME / 2.0);
        assert!(arm.out > 0.3 && arm.out < 0.7, "{}", arm.out);
        run(&mut arm, true, SWING_TIME);
        assert_eq!(arm.out, 1.0);
        run(&mut arm, false, 5.0);
        assert_eq!(arm.out, 1.0);

        arm.since_move = 0.0;
        run(&mut arm, true, FOLD_DELAY / 2.0);
        assert_eq!(arm.out, 1.0, "the page is still on its way");
        run(&mut arm, true, FOLD_DELAY + SWING_TIME);
        assert_eq!(arm.out, 0.0);
    }

    #[test]
    fn folded_lies_along_the_side_towards_the_back() {
        let mut arm = Arm::new(0.5);
        arm.step(true, DT);
        let along = arm.rotation() * Vec3::NEG_X;
        assert!(along.distance(Vec3::NEG_Z) < 1e-5, "{along}");
    }
}
