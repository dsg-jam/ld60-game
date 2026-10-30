use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use ld60_game_player::camera::PlayerCamera;
use ld60_game_player::focus::ClickTool;

use super::props::Props;
use ld60_game_device::{Aim, Phillips, Turning};

/// How fast the screwdriver moves to a screw and back, as the angular
/// frequency of a critically damped spring. It settles in about 5 / this
/// seconds.
const STIFFNESS: f32 = 6.0;
/// The stiffness while a screw turns. The tip has only the last bit to go
/// then and must sit in the head before the screw starts to turn.
const SEATED_STIFFNESS: f32 = 14.0;
/// The tip moves at most this fast, in meters per second, like a calm hand.
const MAX_SPEED: f32 = 0.45;
/// The tip turns at most this fast, in radians per second.
const MAX_SPIN: f32 = 3.5;
/// A goal that moves slower than this, in meters per second, is followed
/// without lagging behind. Faster moves are jumps to another screw.
const FOLLOW_SPEED: f32 = 0.2;
/// Longer frames are cut to this, so the spring stays stable.
const MAX_STEP: f32 = 1.0 / 30.0;
/// How far above a hovered screw the tip waits.
const HOVER: f32 = 0.012;
/// The handle points at least this far away from the camera, in radians.
/// Along a screw that faces the player it would poke into the view.
const CLEAR: f32 = 0.9;

/// The screwdriver on the counter. It turns the screws of a device in and
/// out, see [`Turning`], and waits over a screw the pointer is on.
#[derive(Component, Debug)]
pub(super) struct Screwdriver {
    rest: Transform,
    /// Where the tip is, as the head of a screw it sits on. It eases between
    /// the screws and its rest.
    tip: Transform,
    /// How fast the tip moves, in meters per second.
    velocity: Vec3,
    /// How fast the tip turns, as a scaled axis per second.
    spin: Vec3,
    /// Where the goal was in the last frame.
    last_goal: Option<Vec3>,
}

impl Screwdriver {
    pub(super) fn new(rest: Transform) -> Self {
        Self {
            rest,
            tip: rest * Self::tip_to_rest(),
            velocity: Vec3::ZERO,
            spin: Vec3::ZERO,
            last_goal: None,
        }
    }

    /// Moves the tip towards `goal` for `dt` seconds. It starts slowly and
    /// comes to rest on the goal without passing it. A goal that moves
    /// slowly, like a screw being turned out, is followed without lag.
    fn ease(&mut self, goal: &Transform, stiffness: f32, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let goal_speed = self
            .last_goal
            .map(|last| (goal.translation - last) / dt)
            .filter(|speed| speed.length() < FOLLOW_SPEED)
            .unwrap_or_default();
        self.last_goal = Some(goal.translation);
        let dt = dt.min(MAX_STEP);
        let pull = |off: Vec3, speed: Vec3| stiffness * stiffness * off - 2.0 * stiffness * speed;
        let off = goal.translation - self.tip.translation;
        self.velocity += pull(off, self.velocity - goal_speed) * dt;
        self.velocity = self.velocity.clamp_length_max(MAX_SPEED);
        self.tip.translation += self.velocity * dt;
        let mut turn = goal.rotation * self.tip.rotation.inverse();
        // The short way round.
        if turn.w < 0.0 {
            turn = -turn;
        }
        self.spin += pull(turn.to_scaled_axis(), self.spin) * dt;
        self.spin = self.spin.clamp_length_max(MAX_SPIN);
        self.tip.rotation =
            (Quat::from_scaled_axis(self.spin * dt) * self.tip.rotation).normalize();
    }

    /// `screw`, turned around its axis by a quarter turn or more where that
    /// is closer to the tip as it is. The cross of the tip fits the head
    /// all four ways.
    fn fit(&self, screw: Transform) -> Transform {
        let rotation = (0..4)
            .map(|quarter| screw.rotation * Quat::from_rotation_y(quarter as f32 * FRAC_PI_2))
            .max_by(|a, b| {
                let closeness = |rotation: &Quat| rotation.dot(self.tip.rotation).abs();
                closeness(a).total_cmp(&closeness(b))
            })
            .unwrap_or(screw.rotation);
        screw.with_rotation(rotation)
    }

    /// Puts the tip on the screw that turns, if any, and turns with it.
    /// Otherwise it waits over the screw the pointer is on, if any. The
    /// clutter stays at the origin, so its space is the world.
    #[expect(clippy::type_complexity, reason = "Bevy system")]
    pub(super) fn follow(
        time: Res<Time>,
        turning: Query<&Turning>,
        mut places: ParamSet<(
            Aim,
            Single<&Transform, With<PlayerCamera>>,
            Query<(&mut Screwdriver, &mut Transform)>,
        )>,
    ) {
        let screw = turning.iter().next();
        let waiting = places
            .p0()
            .at(ClickTool::Screwdriver)
            .map(|spot| spot * Transform::from_translation(Vec3::NEG_Y * HOVER));
        let camera = places.p1().translation;
        let mut screwdrivers = places.p2();
        let dt = time.delta_secs();
        for (mut screwdriver, mut transform) in &mut screwdrivers {
            let goal = match (screw, waiting) {
                (Some(screw), _) => Self::clear_of(
                    screwdriver
                        .fit(screw.at * Transform::from_translation(Vec3::Y * Phillips::SEAT)),
                    camera,
                ),
                (None, Some(spot)) => Self::clear_of(screwdriver.fit(spot), camera),
                (None, None) => screwdriver.rest * Self::tip_to_rest(),
            };
            let stiffness = if screw.is_some() {
                SEATED_STIFFNESS
            } else {
                STIFFNESS
            };
            screwdriver.ease(&goal, stiffness, dt);
            let angle = screw.map_or(0.0, |screw| screw.angle);
            transform.set_if_neq(
                screwdriver.tip
                    * Transform::from_rotation(Quat::from_rotation_y(angle))
                    * Self::rest_to_tip(),
            );
        }
    }

    /// `tip`, turned around the tip so the handle points at least
    /// [`CLEAR`] away from `camera`. It stays along the screw where it can.
    fn clear_of(tip: Transform, camera: Vec3) -> Transform {
        let handle = tip.rotation * Vec3::NEG_Y;
        let Some(to_camera) = (camera - tip.translation).try_normalize() else {
            return tip;
        };
        let off = handle.angle_between(to_camera);
        if off >= CLEAR {
            return tip;
        }
        // Straight at the camera, any way away will do, so lean up.
        let axis = to_camera
            .cross(handle)
            .try_normalize()
            .or_else(|| to_camera.cross(Vec3::Y).try_normalize())
            .unwrap_or(Vec3::X);
        tip.with_rotation(Quat::from_axis_angle(axis, CLEAR - off) * tip.rotation)
    }

    /// From the space of the screwdriver to the head of a screw: the tip on
    /// the middle of the head and the shaft along +Y, into the screw.
    fn rest_to_tip() -> Transform {
        let rotation = Quat::from_rotation_z(FRAC_PI_2);
        Transform::from_translation(-(rotation * Props::screwdriver_tip())).with_rotation(rotation)
    }

    /// The inverse of [`Self::rest_to_tip`].
    fn tip_to_rest() -> Transform {
        Transform::from_translation(Props::screwdriver_tip())
            .with_rotation(Quat::from_rotation_z(-FRAC_PI_2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_handle_leans_away_from_the_camera_only_when_it_points_there() {
        let camera = Vec3::new(0.0, 0.0, 0.3);
        // The screw faces the camera, so the handle would point at it.
        let facing = Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2));
        let leaned = Screwdriver::clear_of(facing, camera);
        let handle = leaned.rotation * Vec3::NEG_Y;
        assert!(handle.angle_between(camera) >= CLEAR - 1e-4, "{handle}");
        let sideways = Transform::from_rotation(Quat::from_rotation_z(FRAC_PI_2));
        assert_eq!(Screwdriver::clear_of(sideways, camera), sideways);
    }

    #[test]
    fn the_tip_starts_slowly_and_settles_on_the_goal() {
        let mut screwdriver = Screwdriver::new(Transform::IDENTITY);
        let start = screwdriver.tip;
        let goal = Transform::from_xyz(0.1, 0.0, 0.0).with_rotation(Quat::from_rotation_y(1.0));
        let dt = 1.0 / 144.0;
        screwdriver.ease(&goal, STIFFNESS, dt);
        let first = screwdriver.tip.translation.distance(start.translation);
        assert!(first < 0.1 * 0.01, "{first}");
        let mut farthest = 0.0f32;
        for _ in 0..288 {
            screwdriver.ease(&goal, STIFFNESS, dt);
            farthest = farthest.max(screwdriver.tip.translation.x - start.translation.x);
        }
        assert!(farthest <= 0.1 + 1e-4, "{farthest}");
        assert!(screwdriver.tip.translation.distance(goal.translation) < 1e-3);
        assert!(screwdriver.tip.rotation.angle_between(goal.rotation) < 1e-2);
    }

    #[test]
    fn the_tip_turns_the_short_way_round() {
        let rotation = Quat::from_rotation_x(3.0);
        let mut screwdriver = Screwdriver::new(Transform::IDENTITY);
        screwdriver.tip.rotation = rotation;
        // The same rotation, written the other way.
        let goal = screwdriver.tip.with_rotation(-Quat::from_rotation_x(3.1));
        let mut widest = 0.0f32;
        for _ in 0..144 {
            screwdriver.ease(&goal, STIFFNESS, 1.0 / 144.0);
            widest = widest.max(screwdriver.tip.rotation.angle_between(rotation));
        }
        assert!(widest < 0.11, "{widest}");
    }

    #[test]
    fn the_tip_keeps_up_with_a_screw_turning_out() {
        let mut screwdriver = Screwdriver::new(Transform::IDENTITY);
        let dt = 1.0 / 144.0;
        let mut goal = screwdriver.tip;
        let mut behind = 0.0f32;
        for frame in 0..288 {
            goal.translation += Vec3::NEG_Y * 0.01 * dt;
            screwdriver.ease(&goal, SEATED_STIFFNESS, dt);
            if frame > 144 {
                behind = behind.max(screwdriver.tip.translation.distance(goal.translation));
            }
        }
        assert!(behind < 1e-4, "{behind}");
    }
}
