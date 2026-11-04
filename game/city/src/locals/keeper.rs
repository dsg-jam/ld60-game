use std::f32::consts::{PI, TAU};
use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::gaze::Gaze;
use super::limb::{Limb, rod};
use super::sound::{Footfall, Footfalls, Voice};
use super::{Figures, KEEPER_TALK, Pick, Whims};

/// The shell, low and wide, and the plate on top of it.
const SHELL: Vec3 = Vec3::new(0.72, 0.8, 0.62);
const SHELL_HEIGHT: f32 = 0.95;
const PLATE: Vec3 = Vec3::new(0.6, 0.45, 0.5);
const PLATE_AT: Vec3 = Vec3::new(0.0, 1.3, -0.06);
const NECK_BASE: Vec3 = Vec3::new(0.0, 1.18, 0.22);
const HEAD: Vec3 = Vec3::new(0.0, 1.5, 0.34);
const NECK: f32 = 0.08;
const SHOULDER: Vec3 = Vec3::new(0.26, 1.12, 0.2);
const ARM: Limb = Limb {
    upper: 0.28,
    lower: 0.3,
};
const UPPER_ARM: f32 = 0.05;
const FOREARM: f32 = 0.04;
const HAND: Vec3 = Vec3::new(0.07, 0.035, 0.08);
/// Hands on the counter, in the keeper's space: the top is this high,
/// and its edge this far in front.
const COUNTER_TOP: f32 = 0.995;
const REACH: Range<f32> = 0.42..0.66;
/// How high hands lift moving between spots, and how fast they move.
const HAND_LIFT: f32 = 0.06;
const HAND_SPEED: f32 = 0.6;
/// Seconds between moving a hand, and between moving along the counter.
const FIDGET: Range<f32> = 0.8..3.0;
const SHIFT: Range<f32> = 5.0..15.0;
/// Meters per second along the counter, how fast it gets there, and the
/// length of a step.
const SHUFFLE: f32 = 0.3;
const SHUFFLE_ACCELERATION: f32 = 0.6;
const STEP: f32 = 0.16;
/// How far the shell rocks to the side, rises and turns along the
/// counter while shuffling.
const ROCK: f32 = 0.07;
const RISE: f32 = 0.02;
const TURN: f32 = 0.35;
/// Loudness of its steps, which the counter muffles.
const STEP_LOUDNESS: f32 = 0.3;
/// Room left at the ends of the counter.
const COUNTER_END: f32 = 0.45;
const BREATH_RATE: f32 = 0.25;
const BREATH: f32 = 0.015;

/// A keeper behind the counter of a stall, under a heavy shell, with long
/// arms for the goods. Its legs are hidden behind the counter, so it has
/// none.
#[derive(Component, Debug)]
pub(super) struct Keeper {
    size: f32,
    home: Transform,
    /// How far it can move along the counter, from the middle.
    room: f32,
    x: f32,
    toward: f32,
    /// Speed along the counter.
    speed: f32,
    /// Steps taken.
    steps: f32,
    hands: [Vec3; 2],
    aims: [Vec3; 2],
    fidget: f32,
    shift: f32,
    breath: f32,
    rig: Rig,
}

#[derive(Debug)]
struct Rig {
    shell: Entity,
    head: Entity,
    uppers: [Entity; 2],
    forearms: [Entity; 2],
    hands: [Entity; 2],
}

impl Keeper {
    pub(super) fn spawn(
        commands: &mut Commands,
        figures: &Figures,
        rng: &mut impl Rng,
        home: Transform,
        half_width: f32,
    ) {
        let size = rng.random_range(0.9..1.1f32);
        let colors = figures.colors(rng);
        let shell = figures.shells.pick(rng).clone();
        let mut rig = None;
        let mut root = commands.spawn((
            Name::new("keeper"),
            home,
            Visibility::default(),
            Gaze::default(),
        ));
        root.with_children(|parent| {
            let mut body = parent.spawn((Transform::default(), Visibility::default()));
            let mut head = Entity::PLACEHOLDER;
            body.with_children(|body| {
                body.spawn(figures.ball(
                    &shell,
                    Transform::from_xyz(0.0, SHELL_HEIGHT * size, 0.0).with_scale(SHELL * size),
                ));
                body.spawn(figures.ball(
                    &shell,
                    Transform::from_translation(PLATE_AT * size).with_scale(PLATE * size),
                ));
                for index in 0..rng.random_range(2..5) {
                    let at = Vec3::new(0.0, 1.52 - index as f32 * 0.06, -0.08 - index as f32 * 0.1);
                    body.spawn(figures.ball(
                        &shell,
                        Transform::from_translation(at * size).with_scale(Vec3::splat(0.07 * size)),
                    ));
                }
                body.spawn(figures.rod(
                    &colors.skin,
                    rod(NECK_BASE * size, HEAD * size, NECK * size),
                ));
                head = body
                    .spawn((
                        Transform::from_translation(HEAD * size),
                        Visibility::default(),
                    ))
                    .with_children(|head| {
                        let feelers = rng.random_bool(0.5);
                        figures.spawn_head(head, &colors, size, feelers, rng);
                    })
                    .id();
            });
            let shell = body.id();
            let uppers = [(); 2].map(|()| parent.spawn(figures.rod(&colors.skin, default())).id());
            let forearms =
                [(); 2].map(|()| parent.spawn(figures.rod(&colors.skin, default())).id());
            let hands = [(); 2].map(|()| parent.spawn(figures.ball(&colors.skin, default())).id());
            rig = Some(Rig {
                shell,
                head,
                uppers,
                forearms,
                hands,
            });
        });
        let Some(rig) = rig else {
            unreachable!("with_children runs at once");
        };
        let rest = [-1.0, 1.0].map(|side| Vec3::new(side * 0.22, COUNTER_TOP, 0.5));
        root.insert((
            Voice::new(size, KEEPER_TALK, rng),
            Self {
                size,
                home,
                room: (half_width - COUNTER_END).max(0.0),
                x: 0.0,
                toward: 0.0,
                speed: 0.0,
                steps: 0.0,
                hands: rest,
                aims: rest,
                fidget: rng.random_range(FIDGET),
                shift: rng.random_range(SHIFT),
                breath: rng.random_range(0.0..TAU),
                rig,
            },
        ));
    }

    /// Moves the hands over the goods and the keeper along the counter.
    pub(super) fn work(
        time: Res<Time>,
        mut whims: ResMut<Whims>,
        mut footfalls: ResMut<Footfalls>,
        mut keepers: Query<(&mut Keeper, &mut Transform)>,
    ) {
        let dt = time.delta_secs();
        let rng = &mut whims.0;
        for (mut keeper, mut transform) in &mut keepers {
            keeper.fidget -= dt;
            if keeper.fidget <= 0.0 {
                keeper.fidget = rng.random_range(FIDGET);
                let hand = usize::from(rng.random_bool(0.5));
                let side = if hand == 0 { -1.0 } else { 1.0 };
                keeper.aims[hand] = Vec3::new(
                    side * rng.random_range(0.05..0.4),
                    COUNTER_TOP,
                    rng.random_range(REACH),
                );
            }
            keeper.shift -= dt;
            if keeper.shift <= 0.0 {
                keeper.shift = rng.random_range(SHIFT);
                keeper.toward = rng.random_range(-1.0..=1.0) * keeper.room;
            }
            let target = ((keeper.toward - keeper.x) * 2.0).clamp(-SHUFFLE, SHUFFLE);
            let change = SHUFFLE_ACCELERATION * dt;
            keeper.speed += (target - keeper.speed).clamp(-change, change);
            let moved = keeper.speed * dt;
            keeper.x += moved;
            let before = keeper.steps;
            keeper.steps += moved.abs() / STEP;
            if keeper.steps.floor() > before.floor() {
                let side = if (keeper.steps as u32).is_multiple_of(2) {
                    -0.15
                } else {
                    0.15
                };
                let at = keeper
                    .home
                    .transform_point(Vec3::new(keeper.x + side, 0.0, 0.0));
                footfalls.0.push(Footfall {
                    at,
                    level: STEP_LOUDNESS * keeper.size,
                    indoors: true,
                });
            }
            for hand in 0..2 {
                let aim = keeper.aims[hand] - Vec3::X * moved;
                keeper.aims[hand] = aim;
                let at = keeper.hands[hand];
                let flat = (aim - at).with_y(0.0);
                let move_by = (HAND_SPEED * dt).min(flat.length());
                keeper.hands[hand] = (at + flat.normalize_or_zero() * move_by).with_y(aim.y);
            }
            *transform = keeper.home * Transform::from_xyz(keeper.x, 0.0, 0.0);
        }
    }

    pub(super) fn pose(
        time: Res<Time>,
        mut keepers: Query<(&mut Keeper, &Gaze)>,
        mut parts: Query<&mut Transform, Without<Keeper>>,
    ) {
        let dt = time.delta_secs();
        for (mut keeper, gaze) in &mut keepers {
            keeper.breath += dt * BREATH_RATE * TAU;
            let size = keeper.size;
            let breath = 1.0 + BREATH * keeper.breath.sin();
            let gait = (keeper.speed.abs() / SHUFFLE).min(1.0);
            let rock = (PI * keeper.steps).sin();
            let waddle = Transform::from_xyz(0.0, RISE * size * gait * rock.abs(), 0.0)
                .with_rotation(
                    Quat::from_rotation_y(TURN * keeper.speed / SHUFFLE)
                        * Quat::from_rotation_z(ROCK * gait * rock),
                );
            let rig = &keeper.rig;
            let mut set = |entity: Entity, transform: Transform| {
                if let Ok(mut part) = parts.get_mut(entity) {
                    *part = transform;
                }
            };
            set(
                rig.shell,
                waddle.with_scale(Vec3::new(breath, 1.0 / breath.sqrt(), breath)),
            );
            set(
                rig.head,
                Transform::from_translation(HEAD * size).with_rotation(
                    Quat::from_rotation_y(gaze.turn.x) * Quat::from_rotation_x(gaze.turn.y),
                ),
            );
            for (index, side) in [-1.0f32, 1.0].into_iter().enumerate() {
                let shoulder = waddle.transform_point(SHOULDER * Vec3::new(side, 1.0, 1.0) * size);
                let hand = keeper.hands[index];
                let distance = (keeper.aims[index] - hand).with_y(0.0).length();
                let lifted = hand + Vec3::Y * (HAND_LIFT * (distance / 0.1).min(1.0) + HAND.y);
                let bend = Vec3::new(side * 0.7, -1.0, -0.4);
                let (elbow, wrist) = (ARM * size).reach(shoulder, lifted, bend);
                set(rig.uppers[index], rod(shoulder, elbow, UPPER_ARM * size));
                set(rig.forearms[index], rod(elbow, wrist, FOREARM * size));
                set(
                    rig.hands[index],
                    Transform::from_translation(wrist).with_scale(HAND * size),
                );
            }
        }
    }
}
