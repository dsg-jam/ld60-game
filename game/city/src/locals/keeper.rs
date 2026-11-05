use std::f32::consts::TAU;
use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::bird::{Bird, Birth, Carry, Dress, Posture, Work};
use super::gaze::Gaze;
use super::plumage::Coat;
use super::sound::{Footfall, Footfalls};
use super::walk::Customers;
use super::{Figures, PlumageMaterial, Whims};

/// Keepers stand on a crate behind the counter, this high and this far
/// towards it from the spot behind it, to reach over it.
const CRATE: f32 = 0.18;
const FORWARD: f32 = 0.25;
/// Hands on the counter, in the keeper's space: the top is this high,
/// and the goods lie this far in front.
const COUNTER_TOP: f32 = 0.995 - CRATE + 0.02;
const REACH: Range<f32> = 0.2..0.4;
/// How high hands lift moving between spots, and how fast they move.
const HAND_LIFT: f32 = 0.06;
const HAND_SPEED: f32 = 0.5;
/// Seconds between moving a hand, and between moving along the counter.
const FIDGET: Range<f32> = 0.8..3.0;
const SHIFT: Range<f32> = 5.0..15.0;
/// Meters per second along the counter, how fast it gets there, and the
/// length of a step.
const SHUFFLE: f32 = 0.3;
const SHUFFLE_ACCELERATION: f32 = 0.6;
const STEP: f32 = 0.12;
/// Loudness of its steps, which the counter muffles.
const STEP_LOUDNESS: f32 = 0.3;
/// Room left at the ends of the counter.
const COUNTER_END: f32 = 0.45;
/// Customers this far in front of the counter, and this far past its
/// ends, get served.
const SERVES: std::ops::Range<f32> = 0.2..2.5;
const SERVES_PAST: f32 = 0.8;
/// Serving, keepers hold out goods this far to the customer's side.
const OFFER: f32 = 0.3;
/// With no one to serve, keepers wipe the counter now and then, with this
/// chance per second, for this many seconds, in loops this wide and deep,
/// this many per second.
const WIPE_CHANCE: f32 = 0.05;
const WIPE_FOR: Range<f32> = 3.0..7.0;
const WIPE: Vec2 = Vec2::new(0.08, 0.035);
const WIPE_RATE: f32 = 1.2;
/// The rag lies this far under the hand.
const RAG_DOWN: f32 = 0.02;

/// A bird keeping a stall: it shuffles along the counter and moves its
/// hands over the goods.
#[derive(Component, Debug)]
pub(super) struct Keeper {
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
    wipe: Option<Wipe>,
    rag: Entity,
}

/// A keeper wiping the counter with one hand, round a spot in its space.
#[derive(Debug, Clone, Copy)]
struct Wipe {
    hand: usize,
    middle: Vec3,
    left: f32,
    turn: f32,
}

impl Keeper {
    pub(super) fn spawn(
        commands: &mut Commands,
        figures: &Figures,
        plumages: &mut Assets<PlumageMaterial>,
        rng: &mut impl Rng,
        home: Transform,
        half_width: f32,
    ) {
        let home = home * Transform::from_xyz(0.0, CRATE, FORWARD);
        let birth = Birth {
            at: home,
            posture: Posture::Keeping,
            carry: Carry::Never,
            dress: Dress::pick(rng),
            sheltered: true,
            coat: Coat::new(plumages, &figures.bare, rng),
        };
        let bird = Bird::spawn(commands, figures, rng, birth);
        let rest = [-1.0, 1.0].map(|side| Vec3::new(side * 0.15, COUNTER_TOP, REACH.start));
        let rag = commands.spawn((figures.birds.rag(rng), ChildOf(bird))).id();
        commands.entity(bird).insert((
            Name::new("keeper"),
            Self {
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
                wipe: None,
                rag,
            },
        ));
    }

    /// The nearest customer in front of its counter, in its home's space.
    fn customer(&self, customers: &Customers) -> Option<Vec3> {
        let home = self.home.to_matrix().inverse();
        customers
            .0
            .iter()
            .map(|at| home.transform_point3(*at))
            .filter(|at| SERVES.contains(&at.z) && at.x.abs() < self.room + SERVES_PAST)
            .min_by(|a, b| (a.x - self.x).abs().total_cmp(&(b.x - self.x).abs()))
    }

    /// What its hands and feet do, for a bird of `size`.
    pub(super) fn work(&self, size: f32) -> Work {
        Work {
            hands: [0, 1].map(|hand| {
                let distance = (self.aims[hand] - self.hands[hand]).with_y(0.0).length();
                let lift = HAND_LIFT * (distance / 0.1).min(1.0);
                (self.hands[hand] + Vec3::Y * lift) / size
            }),
            shuffle: (self.speed.abs() / SHUFFLE).min(1.0),
            steps: self.steps,
        }
    }

    /// Moves the hands over the goods and the keeper along the counter.
    pub(super) fn work_on(
        time: Res<Time>,
        mut whims: ResMut<Whims>,
        mut footfalls: ResMut<Footfalls>,
        customers: Res<Customers>,
        mut keepers: Query<(&mut Keeper, &Bird, &mut Gaze, &mut Transform)>,
        mut rags: Query<(&mut Transform, &mut Visibility), Without<Keeper>>,
    ) {
        let dt = time.delta_secs();
        let rng = &mut whims.0;
        for (mut keeper, bird, mut gaze, mut transform) in &mut keepers {
            let customer = keeper.customer(&customers);
            keeper.fidget -= dt;
            if keeper.fidget <= 0.0 {
                keeper.fidget = rng.random_range(FIDGET);
                let hand = usize::from(rng.random_bool(0.5));
                let side = if hand == 0 { -1.0 } else { 1.0 };
                let x = match customer {
                    // Holds something out towards the customer.
                    Some(at) => (at.x - keeper.x).clamp(-OFFER, OFFER) + side * 0.05,
                    None => side * rng.random_range(0.05..0.3),
                };
                let reach = match customer {
                    Some(_) => REACH.end,
                    None => rng.random_range(REACH),
                };
                keeper.aims[hand] = Vec3::new(x, COUNTER_TOP, reach);
            }
            keeper.wipe_on(customer.is_some(), &mut gaze, dt, rng);
            keeper.shift -= dt;
            if let Some(at) = customer {
                keeper.toward = at.x.clamp(-keeper.room, keeper.room);
                let turn = (at.x - keeper.x).atan2(at.z);
                gaze.glance_at(Vec2::new(turn, 0.15), 0.5);
            } else if keeper.shift <= 0.0 {
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
                    -0.1
                } else {
                    0.1
                };
                let at = keeper
                    .home
                    .transform_point(Vec3::new(keeper.x + side, -CRATE, 0.0));
                footfalls.0.push(Footfall {
                    at,
                    level: STEP_LOUDNESS * bird.size(),
                    indoors: true,
                });
            }
            for hand in 0..2 {
                let aim = keeper.aims[hand] - Vec3::X * moved;
                keeper.aims[hand] = aim;
                let at = keeper.hands[hand];
                let flat = (aim - at).with_y(0.0);
                let speed = if keeper.wipe.is_some_and(|wipe| wipe.hand == hand) {
                    2.0 * HAND_SPEED
                } else {
                    HAND_SPEED
                };
                let move_by = (speed * dt).min(flat.length());
                keeper.hands[hand] = (at + flat.normalize_or_zero() * move_by).with_y(aim.y);
            }
            if let Ok((mut rag, mut visibility)) = rags.get_mut(keeper.rag) {
                match keeper.wipe {
                    Some(wipe) => {
                        visibility.set_if_neq(Visibility::Inherited);
                        let hand = keeper.hands[wipe.hand];
                        *rag = Transform::from_translation(hand - Vec3::Y * RAG_DOWN)
                            .with_rotation(Quat::from_rotation_y(wipe.turn * 0.3));
                    }
                    None => {
                        visibility.set_if_neq(Visibility::Hidden);
                    }
                }
            }
            *transform = keeper.home * Transform::from_xyz(keeper.x, 0.0, 0.0);
        }
    }

    /// Wipes the counter now and then while no one is `served`, round and
    /// round a spot with one hand, looking down at it.
    fn wipe_on(&mut self, served: bool, gaze: &mut Gaze, dt: f32, rng: &mut impl Rng) {
        if served {
            self.wipe = None;
            return;
        }
        let chance = f64::from((WIPE_CHANCE * dt).clamp(0.0, 1.0));
        if self.wipe.is_none() && rng.random_bool(chance) {
            let hand = usize::from(rng.random_bool(0.5));
            let side = if hand == 0 { -1.0 } else { 1.0 };
            self.wipe = Some(Wipe {
                hand,
                middle: Vec3::new(
                    side * rng.random_range(0.05..0.2),
                    COUNTER_TOP,
                    REACH.start + WIPE.y,
                ),
                left: rng.random_range(WIPE_FOR),
                turn: 0.0,
            });
        }
        let Some(mut wipe) = self.wipe else {
            return;
        };
        // Stays where it is meanwhile, with the other hand still.
        self.toward = self.x;
        self.shift = self.shift.max(0.5);
        self.fidget = self.fidget.max(0.5);
        wipe.turn += WIPE_RATE * TAU * dt;
        wipe.left -= dt;
        let round = Vec3::new(wipe.turn.cos() * WIPE.x, 0.0, wipe.turn.sin() * WIPE.y);
        self.aims[wipe.hand] = wipe.middle + round;
        let at = wipe.middle;
        gaze.glance_at(Vec2::new(at.x.atan2(at.z), 0.6), 0.5);
        self.wipe = (wipe.left > 0.0).then_some(wipe);
    }
}
