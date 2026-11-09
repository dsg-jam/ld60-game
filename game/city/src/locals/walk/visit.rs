use std::f32::consts::TAU;
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
use super::outing::{Activity, BESIDE, SIT_TIME, ease, turned};
use super::{SIDEWALKS, Walker};

/// Someone takes an errand when it is this many meters ahead of where it
/// goes, out of sight of the shop.
const TAKE_WITHIN: Range<f32> = 8.0..18.0;
/// Having left, a visitor is out of sight this far from the counter.
const GONE: f32 = 12.0;
/// Seconds to hold the hand out, and to draw it back.
const REACH_TIME: f32 = 1.2;
const WITHDRAW_TIME: f32 = 0.6;
/// Calling, a visitor waves for this many seconds out of every this many.
const WAVE_FOR: f32 = 1.8;
const CALL_EVERY: f32 = 4.5;
/// Waving, the hand goes up this high above the eyes, this far out to the
/// side and in front, and swings this far this many times a second.
const WAVE_UP: f32 = 0.12;
const WAVE_OUT: Vec2 = Vec2::new(0.22, 0.3);
const WAVE_SWING: f32 = 0.09;
const WAVE_RATE: f32 = 2.2;
/// Urging, the held out hand shakes this far this many times a second.
const SHAKE: f32 = 0.025;
const SHAKE_RATE: f32 = 3.5;
/// How fast visitors turn to the counter, per second.
const TURN_RATE: f32 = 4.0;
/// Eyes are this high for each meter of size.
const EYES: f32 = 1.15;
/// Seated, a visitor looks at the player's hands, and at the player for
/// this many seconds now and then.
const WATCH_FOR: Range<f32> = 3.0..8.0;
const LOOK_UP_FOR: Range<f32> = 0.8..1.6;

/// Calls someone passing by to the shop counter, to stand at `stand` in
/// front of it, carrying `carrying`. `errand` is the shop's own name for
/// the visit, found again on the [`Visitor`].
#[derive(Event, Clone, Copy, Debug)]
pub struct Summon {
    pub errand: Entity,
    pub stand: Vec2,
    pub carrying: Entity,
}

/// The visitor of `errand` left and is out of sight.
#[derive(Event, Clone, Copy, Debug)]
pub struct Left {
    pub errand: Entity,
}

/// Someone at the shop on an errand. The shop says what it does with
/// `ask`. What it is `carrying` moves with its left hand.
#[derive(Component, Debug)]
pub struct Visitor {
    pub errand: Entity,
    pub ask: Ask,
    pub carrying: Option<Entity>,
    stand: Vec2,
    at_counter: bool,
    /// The stool it sits on, as far as it sat down.
    seat: Option<(Vec3, f32)>,
    sitting: f32,
    /// Where the hand goes, and how far it is there.
    hand: Vec3,
    reach: f32,
    /// Seconds into calling or urging.
    clock: f32,
    /// Seconds until it looks somewhere else, while seated.
    look: f32,
    at_player: bool,
}

/// What the shop asks a [`Visitor`] to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ask {
    /// Stand at the counter and look at the player.
    Wait,
    /// Wave what it carries, or its hand, and call out now and then.
    Call,
    /// Hold the hand out to `at` over the counter, with what it carries or
    /// empty to take something. Shake it and call out while `urge`.
    HoldOut { at: Vec3, urge: bool },
    /// Sit on the stool with its foot at `at`, `height` high, and watch.
    Sit { at: Vec3, height: f32 },
    /// Walk on, out of sight.
    Leave,
}

/// Summons nobody has taken yet.
#[derive(Resource, Debug, Default)]
pub(in super::super) struct Errands {
    waiting: Vec<Summon>,
}

impl Errands {
    pub(in super::super) fn ask(summon: On<Summon>, mut errands: ResMut<Errands>) {
        errands.waiting.push(*summon);
    }

    /// Moves what visitors carry along with their hands, and lets them go
    /// once out of sight.
    pub(in super::super) fn carry(
        mut visitors: Query<(Entity, &Walker, &mut Bird, &Visitor)>,
        mut items: Query<&mut Transform, Without<Walker>>,
        mut commands: Commands,
    ) {
        for (entity, walker, mut bird, visitor) in &mut visitors {
            if let (Some(held), Some(mut item)) = (
                bird.held(),
                visitor.carrying.and_then(|item| items.get_mut(item).ok()),
            ) {
                *item = held;
            }
            let away = (walker.x - visitor.stand.x).abs();
            if visitor.ask == Ask::Leave && !walker.activity.is_visit() && away > GONE {
                bird.carry(false);
                commands.entity(entity).remove::<Visitor>();
                commands.trigger(Left {
                    errand: visitor.errand,
                });
            }
        }
    }

    /// Hands a waiting summons to `walker` if it comes along the sidewalk
    /// by the shop, out of sight still.
    pub(super) fn hand_out(
        &mut self,
        entity: Entity,
        walker: &mut Walker,
        commands: &mut Commands,
    ) {
        let Some(summon) = self.waiting.first().copied() else {
            return;
        };
        if walker.takes(&summon) {
            self.waiting.remove(0);
            walker.activity = Activity::Visit;
            commands.entity(entity).insert(Visitor {
                errand: summon.errand,
                ask: Ask::Wait,
                carrying: Some(summon.carrying),
                stand: summon.stand,
                at_counter: false,
                seat: None,
                sitting: 0.0,
                hand: Vec3::ZERO,
                reach: 0.0,
                clock: 0.0,
                look: 0.0,
                at_player: false,
            });
        }
    }
}

impl Visitor {
    /// Whether it stands at the counter.
    pub fn at_counter(&self) -> bool {
        self.at_counter
    }

    /// Whether it sits on its stool.
    pub fn seated(&self) -> bool {
        self.sitting >= 1.0
    }

    /// Whether the hand is all the way out.
    pub fn holds_out(&self) -> bool {
        matches!(self.ask, Ask::HoldOut { .. }) && self.reach >= 1.0
    }
}

impl Walker {
    /// Whether it would take `summon` now.
    fn takes(&self, summon: &Summon) -> bool {
        let ahead = (summon.stand.x - self.x) * self.heading;
        ptr::eq(self.sidewalk, &SIDEWALKS[0]) && TAKE_WITHIN.contains(&ahead)
    }

    /// Walks up to the counter and does what the shop asks there.
    pub(super) fn visit(
        &mut self,
        visitor: &mut Visitor,
        bird: &mut Bird,
        gaze: &mut Gaze,
        voice: &mut Voice,
        dt: f32,
        rng: &mut impl Rng,
    ) {
        bird.carry(visitor.carrying.is_some());
        visitor.at_counter =
            visitor.seat.is_none() && Vec2::new(self.x, self.z).distance(visitor.stand) < 0.05;
        let counter = CounterLayout::new();
        let player = counter.seated_view().translation;
        if let Some((stool, height)) = visitor.seat {
            let staying = visitor.ask == (Ask::Sit { at: stool, height });
            self.sit_on(visitor, bird, stool, height, staying, dt);
            if visitor.seated() {
                self.watch(visitor, bird, gaze, &counter, dt, rng);
            } else {
                gaze.glance_at(self.look_at(player, bird), 0.5);
            }
            return;
        }
        match visitor.ask {
            Ask::Leave => {
                bird.stop_reaching();
                bird.stand_under(false);
                voice.talk_every(WALKER_TALK);
                let heading = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                self.walk_on(heading, rng);
                return;
            }
            Ask::Sit { at, height } if visitor.reach <= 0.0 => {
                bird.stand_under(true);
                let beside = Self::beside(at);
                if self.walk_to(bird, beside, bird.pace, dt) {
                    visitor.seat = Some((at, height));
                }
                gaze.glance_at(self.look_at(at, bird), 0.5);
                return;
            }
            _ => {}
        }
        if !visitor.at_counter {
            gaze.glance_at(self.look_at(player, bird), 0.5);
            self.walk_to(bird, visitor.stand, bird.pace, dt);
            return;
        }
        // The shop's roof keeps the rain off at the counter.
        bird.stand_under(true);
        bird.speed = 0.0;
        self.yaw = turned(self.yaw, 0.0, TURN_RATE * dt);
        gaze.glance_at(self.look_at(player, bird), 0.5);
        self.gesture(visitor, bird, voice, dt);
    }

    /// Moves the free hand as the shop asks, at the counter. An umbrella
    /// closes first, to have a hand free.
    fn gesture(&self, visitor: &mut Visitor, bird: &mut Bird, voice: &mut Voice, dt: f32) {
        let root = self.place();
        let free = !bird.has_umbrella_open();
        visitor.clock += dt;
        let (goal, out) = match visitor.ask {
            Ask::Call => {
                let into = visitor.clock % CALL_EVERY;
                if into < dt {
                    voice.speak_now();
                }
                let swing = WAVE_SWING * (into * WAVE_RATE * TAU).sin();
                let up = EYES * bird.size() + WAVE_UP;
                let wave = root.transform_point(Vec3::new(WAVE_OUT.x + swing, up, WAVE_OUT.y));
                (wave, into < WAVE_FOR)
            }
            Ask::HoldOut { at, urge } => {
                let shake = if urge {
                    if visitor.clock % CALL_EVERY < dt {
                        voice.speak_now();
                    }
                    Vec3::X * SHAKE * (visitor.clock * SHAKE_RATE * TAU).sin()
                } else {
                    Vec3::ZERO
                };
                (at + shake, true)
            }
            Ask::Wait | Ask::Sit { .. } | Ask::Leave => (visitor.hand, false),
        };
        visitor.hand = goal;
        visitor.reach = if out && free {
            visitor.reach + dt / REACH_TIME
        } else {
            visitor.reach - dt / WITHDRAW_TIME
        }
        .clamp(0.0, 1.0);
        if visitor.reach > 0.0 {
            bird.reach_to(&root, visitor.hand, ease(visitor.reach));
        } else {
            bird.stop_reaching();
        }
        voice.chat(CHAT_TALK);
    }

    /// Sits down on the stool at `stool`, or gets up from it unless
    /// `staying`.
    fn sit_on(
        &mut self,
        visitor: &mut Visitor,
        bird: &mut Bird,
        stool: Vec3,
        height: f32,
        staying: bool,
        dt: f32,
    ) {
        bird.speed = 0.0;
        let ready = self.yaw.abs() < 0.08;
        if !staying {
            visitor.sitting -= dt / SIT_TIME;
        } else if visitor.sitting > 0.0 || ready {
            visitor.sitting += dt / SIT_TIME;
        }
        visitor.sitting = visitor.sitting.clamp(0.0, 1.0);
        let (_, spring, _) = Bird::hop(visitor.sitting);
        let at = Self::beside(stool).lerp(stool.xz(), ease(spring));
        self.x = at.x;
        self.z = at.y;
        self.yaw = turned(self.yaw, 0.0, TURN_RATE * dt);
        bird.sit(height, visitor.sitting, true);
        // The shop's roof keeps the rain off its stools too.
        bird.stand_under(true);
        if !staying && visitor.sitting <= 0.0 {
            visitor.seat = None;
        }
    }

    /// Seated, it watches the player's hands and looks up now and then.
    fn watch(
        &self,
        visitor: &mut Visitor,
        bird: &Bird,
        gaze: &mut Gaze,
        counter: &CounterLayout,
        dt: f32,
        rng: &mut impl Rng,
    ) {
        visitor.look -= dt;
        if visitor.look > 0.0 {
            return;
        }
        visitor.at_player = !visitor.at_player;
        let (target, look) = if visitor.at_player {
            (
                counter.seated_view().translation,
                rng.random_range(LOOK_UP_FOR),
            )
        } else {
            (
                counter.mat.top_spot(Vec2::ZERO),
                rng.random_range(WATCH_FOR),
            )
        };
        visitor.look = look;
        gaze.glance_at(self.look_at(target, bird), look);
    }

    /// Where to stand to sit down on the stool at `stool`, on its outer
    /// side.
    fn beside(stool: Vec3) -> Vec2 {
        stool.xz() + Vec2::X * stool.x.signum() * BESIDE
    }

    /// The turn of the head to look at `at` in the world.
    fn look_at(&self, at: Vec3, bird: &Bird) -> Vec2 {
        let way = at.xz() - Vec2::new(self.x, self.z);
        let local = Quat::from_rotation_y(-self.yaw) * Vec3::new(way.x, 0.0, way.y);
        let down = EYES * bird.size() - at.y;
        Vec2::new(local.x.atan2(local.z), down.atan2(way.length()))
    }
}
