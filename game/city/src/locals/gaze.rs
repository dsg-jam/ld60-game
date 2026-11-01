use bevy::prelude::*;
use ld60_game_player::camera::PlayerCamera;
use rand::RngExt;

use super::super::traffic::Vehicle;
use super::Whims;

/// How far the head turns from the front, and up or down, in radians.
const YAW: f32 = 1.4;
const PITCH: f32 = 0.6;
/// How quickly the head follows where it wants to look, per second.
const FOLLOW: f32 = 6.0;
/// People look at the player within this distance, and at cars within the
/// other.
const PLAYER_NEAR: f32 = 7.0;
const CAR_NEAR: f32 = 12.0;
/// Height of the eyes above the feet, for aiming, before size.
const EYES: f32 = 1.6;

/// Where a head looks, relative to its body.
#[derive(Component, Debug, Default)]
pub(super) struct Gaze {
    /// The head's turn now and where it turns to, as yaw and pitch. A
    /// positive pitch looks down.
    pub(super) turn: Vec2,
    focus: Focus,
    /// Seconds until it looks at something else.
    until: f32,
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
            }
            let aim = match gaze.focus {
                Focus::Ahead => Vec2::new(0.0, 0.05),
                Focus::Turn(turn) => turn,
                Focus::Player => player.map_or(Vec2::ZERO, |at| Self::turn_to(body, eyes, at)),
                Focus::Car(car) => vehicles.get(car).map_or(Vec2::ZERO, |(_, car)| {
                    Self::turn_to(body, eyes, car.translation())
                }),
            };
            let aim = aim.clamp(Vec2::new(-YAW, -PITCH), Vec2::new(YAW, PITCH));
            gaze.turn = gaze.turn.lerp(aim, 1.0 - (-FOLLOW * dt).exp());
        }
    }

    /// The yaw and pitch from `eyes` on `body` to `target`.
    fn turn_to(body: &GlobalTransform, eyes: Vec3, target: Vec3) -> Vec2 {
        let local = body.rotation().inverse() * (target - eyes);
        Vec2::new(local.x.atan2(local.z), -local.y.atan2(local.xz().length()))
    }
}
