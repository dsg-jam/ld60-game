use bevy::prelude::*;
use ld60_game_player::camera::PlayerCamera;
use rand::RngExt;

use super::super::traffic::Vehicle;
use super::Whims;

/// How far the head turns from the front, and up or down, in radians.
pub(super) const YAW: f32 = 1.4;
const PITCH: f32 = 0.6;
/// How quickly the head follows where it wants to look, per second.
const FOLLOW: f32 = 6.0;
/// People look at the player within this distance, and at cars within the
/// other.
const PLAYER_NEAR: f32 = 7.0;
const CAR_NEAR: f32 = 12.0;
/// Height of the eyes above the feet, for aiming, before size.
const EYES: f32 = 1.55;
/// The eyes sit on the sides of the head, so it turns this far away from
/// what it looks at closely, to see it with one eye. It rolls the head to
/// follow up and down with that eye, this much of the pitch.
const EYE_SIDE: f32 = 1.0;
const EYE_ROLL: f32 = 0.8;
/// It looks with the other eye every so many seconds.
const SWITCH_EYES: std::ops::Range<f32> = 2.5..6.0;

/// Where a head looks, relative to its body.
#[derive(Component, Debug, Default)]
pub(super) struct Gaze {
    /// The head's turn now and where it turns to, as yaw and pitch. A
    /// positive pitch looks down.
    pub(super) turn: Vec2,
    /// How far the head rolls, positive to the right.
    pub(super) roll: f32,
    focus: Focus,
    /// Seconds until it looks at something else.
    until: f32,
    /// Which eye it looks with, -1 for the left and 1 for the right, and
    /// for how many seconds more. 0 looks ahead with both.
    eye: f32,
    until_switch: f32,
}

#[derive(Debug, Default, Clone, Copy)]
enum Focus {
    #[default]
    Ahead,
    /// A fixed turn, like a glance at a stall.
    Turn(Vec2),
    Player,
    Car(Entity),
}

impl Gaze {
    /// Looks to `turn` for `seconds`, like at a stall while standing there.
    pub(super) fn glance_at(&mut self, turn: Vec2, seconds: f32) {
        self.focus = Focus::Turn(turn);
        self.until = seconds;
        self.eye = 0.0;
    }

    /// Looks closely at what is at `turn` for `seconds`, with one eye.
    pub(super) fn eye_at(&mut self, turn: Vec2, seconds: f32) {
        self.focus = Focus::Turn(turn);
        self.until = seconds;
        if self.eye == 0.0 {
            self.eye = if turn.x > 0.0 { -1.0 } else { 1.0 };
            self.until_switch = 0.0;
        }
    }

    pub(super) fn glance(
        time: Res<Time>,
        mut whims: ResMut<Whims>,
        player: Query<&GlobalTransform, With<PlayerCamera>>,
        vehicles: Query<(Entity, &GlobalTransform), With<Vehicle>>,
        mut gazes: Query<(&mut Gaze, &GlobalTransform)>,
    ) {
        let dt = time.delta_secs();
        let rng = &mut whims.0;
        let player = player.iter().next().map(GlobalTransform::translation);
        for (mut gaze, body) in &mut gazes {
            let eyes = body.transform_point(Vec3::Y * EYES);
            gaze.until -= dt;
            if gaze.until <= 0.0 {
                let near_car = vehicles
                    .iter()
                    .map(|(entity, car)| (entity, car.translation().distance(eyes)))
                    .filter(|&(_, distance)| distance < CAR_NEAR)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                let sees_player = player.is_some_and(|at| {
                    at.distance(eyes) < PLAYER_NEAR && Self::turn_to(body, eyes, at).x.abs() < YAW
                });
                gaze.focus = match rng.random_range(0.0..1.0) {
                    x if x < 0.35 && sees_player => Focus::Player,
                    x if x < 0.65 && near_car.is_some() => {
                        near_car.map_or(Focus::Ahead, |(car, _)| Focus::Car(car))
                    }
                    x if x < 0.8 => Focus::Ahead,
                    _ => Focus::Turn(Vec2::new(
                        rng.random_range(-1.1..1.1),
                        rng.random_range(-0.2..0.3),
                    )),
                };
                gaze.until = rng.random_range(1.5..4.0);
                // The player is looked at closely, with one eye.
                gaze.eye = match gaze.focus {
                    Focus::Player if rng.random_bool(0.5) => 1.0,
                    Focus::Player => -1.0,
                    Focus::Ahead | Focus::Turn(_) | Focus::Car(_) => 0.0,
                };
            }
            let aim = match gaze.focus {
                Focus::Ahead => Vec2::new(0.0, 0.05),
                Focus::Turn(turn) => turn,
                Focus::Player => player.map_or(Vec2::ZERO, |at| Self::turn_to(body, eyes, at)),
                Focus::Car(car) => vehicles.get(car).map_or(Vec2::ZERO, |(_, car)| {
                    Self::turn_to(body, eyes, car.translation())
                }),
            };
            let roll = if gaze.eye == 0.0 {
                0.0
            } else {
                gaze.until_switch -= dt;
                // Only an eye that the head can turn far enough for.
                let fits = |eye: f32| (aim.x - eye * EYE_SIDE).abs() < YAW;
                if gaze.until_switch <= 0.0 && fits(-gaze.eye) || !fits(gaze.eye) {
                    gaze.eye = -gaze.eye;
                    gaze.until_switch = rng.random_range(SWITCH_EYES);
                }
                -gaze.eye * aim.y * EYE_ROLL
            };
            let aim = Vec2::new(
                aim.x - gaze.eye * EYE_SIDE,
                aim.y * (1.0 - gaze.eye.abs() * EYE_ROLL),
            );
            let aim = aim.clamp(Vec2::new(-YAW, -PITCH), Vec2::new(YAW, PITCH));
            let follow = 1.0 - (-FOLLOW * dt).exp();
            gaze.turn = gaze.turn.lerp(aim, follow);
            gaze.roll += (roll - gaze.roll) * follow;
        }
    }

    /// The yaw and pitch from `eyes` on `body` to `target`.
    fn turn_to(body: &GlobalTransform, eyes: Vec3, target: Vec3) -> Vec2 {
        let local = body.rotation().inverse() * (target - eyes);
        Vec2::new(local.x.atan2(local.z), -local.y.atan2(local.xz().length()))
    }
}
