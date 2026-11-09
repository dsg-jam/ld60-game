use std::ops::Range;
use std::ptr;

use bevy::prelude::*;
use ld60_game_core::layout::counter::CounterLayout;
use rand::{Rng, RngExt};

use super::super::WALKER_TALK;
use super::super::bird::Bird;
use super::super::gaze::Gaze;
use super::super::sound::Voice;
use super::chat::CHAT_TALK;
use super::outing::{Activity, turned};
use super::{SIDEWALKS, Walker};

/// Someone takes an errand when it is this many meters ahead of where it
/// goes, out of sight of the shop.
const TAKE_WITHIN: Range<f32> = 12.0..25.0;
/// Seconds to reach over the counter and put the things down, and to
/// draw the hand back.
const OFFER_TIME: f32 = 1.4;
const WITHDRAW_TIME: f32 = 0.6;
/// Seconds of pointing, looking from the thing to the player and back
/// every so often.
const POINT_FOR: f32 = 4.5;
const LOOK_EVERY: Range<f32> = 0.9..1.6;
/// Pointing, the hand goes this far of the way to a spot this high above
/// the thing.
const POINT_REACH: f32 = 0.8;
const POINT_ABOVE: f32 = 0.12;
/// How fast visitors turn to the counter, per second.
const TURN_RATE: f32 = 4.0;
/// Eyes are this high for each meter of size.
const EYES: f32 = 1.15;

/// Asks someone passing by to bring `items` to the shop counter, stand at
/// `stand` in front of it, put them down at `drop` and point at `point_at`.
/// `items` is an entity at the top level, which the visitor carries and
/// puts down.
#[derive(Event, Clone, Copy, Debug)]
pub struct DropOff {
    pub items: Entity,
    pub stand: Vec2,
    pub drop: Transform,
    pub point_at: Vec3,
}

/// The visitor put the `items` of a [`DropOff`] down on the counter.
#[derive(Event, Clone, Copy, Debug)]
pub struct DroppedOff {
    pub items: Entity,
}

/// Drop-offs nobody has taken yet, and the ones put down this frame.
#[derive(Resource, Debug, Default)]
pub(in super::super) struct Errands {
    waiting: Vec<DropOff>,
    put_down: Vec<DropOff>,
}

/// How far along a visit to the shop counter someone is.
#[derive(Debug, Clone, Copy)]
pub(super) enum Visiting {
    ToCounter,
    /// Reaching over the counter, this far.
    Offer(f32),
    /// Drawing the hand back, this far.
    Withdraw(f32),
    /// Pointing for `left` seconds more, at the player or at the thing,
    /// until `look` runs out.
    Point {
        left: f32,
        look: f32,
        at_player: bool,
    },
}

impl Errands {
    pub(in super::super) fn ask(drop_off: On<DropOff>, mut errands: ResMut<Errands>) {
        errands.waiting.push(*drop_off);
    }

    /// Moves what visitors carry along with their hands, puts down what
    /// they let go of and says so.
    pub(in super::super) fn carry(
        mut errands: ResMut<Errands>,
        walkers: Query<(&Walker, &Bird)>,
        mut items: Query<&mut Transform, Without<Walker>>,
        mut commands: Commands,
    ) {
        for (walker, bird) in &walkers {
            let Activity::Visit {
                errand,
                step: Visiting::ToCounter | Visiting::Offer(_),
            } = walker.activity
            else {
                continue;
            };
            if let (Some(held), Ok(mut item)) = (bird.held(), items.get_mut(errand.items)) {
                *item = held;
            }
        }
        for errand in std::mem::take(&mut errands.put_down) {
            if let Ok(mut item) = items.get_mut(errand.items) {
                *item = errand.drop;
            }
            commands.trigger(DroppedOff {
                items: errand.items,
            });
        }
    }

    /// Puts down what a visitor let go of, once the frame's poses are done.
    pub(super) fn let_go(&mut self, errand: DropOff) {
        self.put_down.push(errand);
    }

    /// Hands a waiting errand to someone coming along the sidewalk by the
    /// shop, out of sight still.
    pub(super) fn hand_out(&mut self, walker: &mut Walker, bird: &mut Bird) {
        let Some(errand) = self.waiting.first().copied() else {
            return;
        };
        if walker.takes(&errand) {
            self.waiting.remove(0);
            bird.carry(true);
            walker.activity = Activity::Visit {
                errand,
                step: Visiting::ToCounter,
            };
        }
    }
}

impl Walker {
    /// Whether it would take `errand` now.
    fn takes(&self, errand: &DropOff) -> bool {
        let ahead = (errand.stand.x - self.x) * self.heading;
        ptr::eq(self.sidewalk, &SIDEWALKS[0]) && TAKE_WITHIN.contains(&ahead)
    }

    /// Walks up to the counter, puts the things down, points at them while
    /// looking at the player, and walks on. Returns the errand once its
    /// things are put down.
    pub(super) fn visit(
        &mut self,
        bird: &mut Bird,
        gaze: &mut Gaze,
        voice: &mut Voice,
        dt: f32,
        rng: &mut impl Rng,
    ) -> Option<DropOff> {
        let Activity::Visit { errand, step } = self.activity else {
            return None;
        };
        let mut put_down = None;
        let root = self.place();
        let drop = errand.drop.translation;
        let step = match step {
            Visiting::ToCounter => {
                gaze.glance_at(self.look_at(drop, bird), 0.5);
                if self.walk_to(bird, errand.stand, bird.pace, dt) {
                    // The shop's roof keeps the rain off at the counter.
                    bird.stand_under(true);
                    Visiting::Offer(0.0)
                } else {
                    Visiting::ToCounter
                }
            }
            Visiting::Offer(done) => {
                bird.speed = 0.0;
                self.yaw = turned(self.yaw, 0.0, TURN_RATE * dt);
                gaze.glance_at(self.look_at(drop, bird), 0.5);
                // An umbrella closes first, to have a hand free.
                let done = if bird.has_umbrella_open() {
                    done
                } else {
                    done + dt / OFFER_TIME
                };
                bird.reach_to(&root, drop, done);
                if done >= 1.0 {
                    bird.carry(false);
                    put_down = Some(errand);
                    Visiting::Withdraw(0.0)
                } else {
                    Visiting::Offer(done)
                }
            }
            Visiting::Withdraw(done) => {
                let done = done + dt / WITHDRAW_TIME;
                bird.reach_to(&root, drop, 1.0 - done);
                if done >= 1.0 {
                    Visiting::Point {
                        left: POINT_FOR,
                        look: 0.0,
                        at_player: false,
                    }
                } else {
                    Visiting::Withdraw(done)
                }
            }
            Visiting::Point {
                left,
                mut look,
                mut at_player,
            } => {
                let at = errand.point_at + Vec3::Y * POINT_ABOVE;
                bird.reach_to(&root, at, POINT_REACH);
                voice.chat(CHAT_TALK);
                look -= dt;
                if look <= 0.0 {
                    look = rng.random_range(LOOK_EVERY);
                    at_player = !at_player;
                    let player = CounterLayout::new().seated_view().translation;
                    let target = if at_player { player } else { errand.point_at };
                    gaze.glance_at(self.look_at(target, bird), look);
                }
                let left = left - dt;
                if left <= 0.0 {
                    bird.stop_reaching();
                    bird.stand_under(false);
                    voice.talk_every(WALKER_TALK);
                    let heading = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                    self.walk_on(heading, rng);
                    return put_down;
                }
                Visiting::Point {
                    left,
                    look,
                    at_player,
                }
            }
        };
        self.activity = Activity::Visit { errand, step };
        put_down
    }

    /// The turn of the head to look at `at` in the world.
    fn look_at(&self, at: Vec3, bird: &Bird) -> Vec2 {
        let way = at.xz() - Vec2::new(self.x, self.z);
        let local = Quat::from_rotation_y(-self.yaw) * Vec3::new(way.x, 0.0, way.y);
        let down = EYES * bird.size() - at.y;
        Vec2::new(local.x.atan2(local.z), down.atan2(way.length()))
    }
}
