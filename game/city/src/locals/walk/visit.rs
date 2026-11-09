use std::f32::consts::{FRAC_PI_2, TAU};
use std::ops::Range;

use bevy::prelude::*;
use ld60_game_core::layout::counter::CounterLayout;
use rand::{Rng, RngExt};

use super::super::bird::Bird;
use super::super::gaze::Gaze;
use super::super::plumage::PlumageMaterial;
use super::super::shadows::Silhouette;
use super::super::sound::Voice;
use super::super::{Figures, WALKER_TALK, Whims};
use super::chat::CHAT_TALK;
use super::outing::{Activity, BESIDE, SIT_TIME, ease, turned};
use super::{ACCELERATION, SIDEWALKS, Walker};

/// A visitor comes along the sidewalk from this far away. Once it has
/// left and is this far away again, it is gone.
const COME_FROM: f32 = 7.0;
/// On the way, it keeps to its side of the sidewalk like everyone else,
/// and turns in to the counter this far before it, along the street.
const TURN_IN: f32 = 1.2;
/// The last steps up to the counter go straight in from this far, at this
/// share of its pace.
const STEP_UP: f32 = 0.5;
const STEP_UP_PACE: f32 = 0.7;
/// Moving along the counter, it steps this far back from it first, at
/// this pace in meters per second, and slows down this close to each
/// step.
const STEP_BACK: f32 = 0.3;
const SHUFFLE_PACE: f32 = 0.5;
const SLOW_DOWN: f32 = 0.25;
const GONE: f32 = 12.0;
/// Seconds to hold the hand out, and to draw it back.
const REACH_TIME: f32 = 1.2;
const WITHDRAW_TIME: f32 = 0.6;
/// Calling, a visitor holds up what it has for this many seconds out of
/// every this many, this far below the eyes, to the side and in front,
/// bobbing it this far this many times a second.
const SHOW_FOR: f32 = 1.8;
const CALL_EVERY: f32 = 4.5;
const SHOW_AT: Vec3 = Vec3::new(0.12, -0.12, 0.38);
const BOB: f32 = 0.025;
const BOB_RATE: f32 = 2.5;
/// How far a visitor leans in, in radians: holding something out over the
/// counter, calling, and shifting about while it waits.
const LEAN_OVER: f32 = 0.2;
const LEAN_CALL: f32 = 0.15;
const FIDGET: Range<f32> = 0.0..0.08;
/// Showing something, a visitor tips it this far towards the player.
const PRESENT: f32 = 0.5;
/// An umbrella closes this far from the counter, and the visitor waits
/// this far away until it is shut, so it does not swing into the counter.
const CLOSE_WITHIN: f32 = 3.5;
const SHUT_BY: f32 = 0.9;
/// Urging, the held out hand shakes this far this many times a second.
const SHAKE: f32 = 0.025;
const SHAKE_RATE: f32 = 3.5;
/// How fast visitors turn to the counter, per second.
const TURN_RATE: f32 = 4.0;
/// Eyes are this high for each meter of size.
const EYES: f32 = 1.15;
/// Seconds a visitor looks at the player, and at something else between.
const AT_PLAYER: Range<f32> = 1.5..4.0;
const ELSEWHERE: Range<f32> = 0.8..2.0;
/// Seated, it watches the player's hands for this many seconds, and looks
/// up at the player for this many.
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

/// The shop no longer needs the visitor of `errand`. It walks on, or
/// never comes.
#[derive(Event, Clone, Copy, Debug)]
pub struct CallOff {
    pub errand: Entity,
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
    /// The middle of its head in the world, after the last pose.
    head: Vec3,
    /// The stool it sits on, as far as it sat down.
    seat: Option<(Vec3, f32)>,
    sitting: f32,
    /// Where the hand goes, and how far it is there.
    hand: Vec3,
    reach: f32,
    /// Seconds into calling or urging.
    clock: f32,
    /// Seconds until it looks somewhere else, and where it looks.
    look: f32,
    at_player: bool,
    gaze: Glance,
    /// How far it leans in, in radians.
    lean: f32,
}

/// Where a visitor looks when not at the player.
#[derive(Debug, Clone, Copy)]
enum Glance {
    /// At what it carries.
    Carried,
    /// Down at the player's work.
    Work,
    /// Off along the street, to this side.
    Street(f32),
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

/// Brings visitors to the shop and sends them away again.
#[derive(Debug)]
pub(in super::super) struct Errands;

impl Errands {
    /// Someone comes along the sidewalk by the shop, on its way to the
    /// counter.
    pub(in super::super) fn summon(
        summon: On<Summon>,
        figures: Res<Figures>,
        mut plumages: ResMut<Assets<PlumageMaterial>>,
        mut whims: ResMut<Whims>,
        mut commands: Commands,
    ) {
        let rng = &mut whims.0;
        let heading = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
        let mut walker = Walker::on(&SIDEWALKS[0], rng);
        walker.heading = heading;
        let lanes = walker.lanes();
        walker.lane = (lanes.start + lanes.end) / 2.0;
        walker.z = walker.lane;
        walker.x = summon.stand.x - heading * COME_FROM;
        walker.yaw = heading * FRAC_PI_2;
        walker.activity = Activity::Visit;
        let entity = figures.spawn_walker(&mut commands, &mut plumages, rng, walker);
        commands.entity(entity).insert(Visitor {
            errand: summon.errand,
            ask: Ask::Wait,
            carrying: Some(summon.carrying),
            stand: summon.stand,
            at_counter: false,
            head: Vec3::ZERO,
            seat: None,
            sitting: 0.0,
            hand: Vec3::ZERO,
            reach: 0.0,
            clock: 0.0,
            look: 0.0,
            at_player: false,
            gaze: Glance::Work,
            lean: 0.0,
        });
    }

    pub(in super::super) fn call_off(call_off: On<CallOff>, mut visitors: Query<&mut Visitor>) {
        for mut visitor in &mut visitors {
            if visitor.errand == call_off.errand {
                visitor.carrying = None;
                visitor.ask = Ask::Leave;
            }
        }
    }

    /// Moves what visitors carry along with their hands, and takes them
    /// away once out of sight.
    pub(in super::super) fn carry(
        mut visitors: Query<(Entity, &Walker, &mut Bird, &mut Visitor, &Silhouette)>,
        mut items: Query<&mut Transform, Without<Walker>>,
        mut commands: Commands,
    ) {
        for (entity, walker, mut bird, mut visitor, silhouette) in &mut visitors {
            visitor.head = silhouette.blobs[1].center;
            if let (Some(held), Some(mut item)) = (
                bird.held(),
                visitor.carrying.and_then(|item| items.get_mut(item).ok()),
            ) {
                *item = held;
            }
            if visitor.ask != Ask::Leave || walker.activity.is_visit() {
                continue;
            }
            // Out from under the roof before the umbrella opens again.
            let place = Vec2::new(walker.x, walker.z);
            bird.stand_under(place.distance(visitor.stand) < CLOSE_WITHIN);
            if (walker.x - visitor.stand.x).abs() > GONE {
                commands.entity(entity).despawn();
                commands.trigger(Left {
                    errand: visitor.errand,
                });
            }
        }
    }
}

impl Visitor {
    /// Whether it stands at the counter.
    pub fn at_counter(&self) -> bool {
        self.at_counter
    }

    /// The middle of its head in the world.
    pub fn head(&self) -> Vec3 {
        self.head
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
        // It leans in and shows things only standing at the counter.
        if !visitor.at_counter {
            bird.lean_in(0.0, dt);
            bird.present(0.0, dt);
        }
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
                bird.lean_in(0.0, f32::INFINITY);
                bird.present(0.0, f32::INFINITY);
                bird.stop_reaching();
                voice.talk_every(WALKER_TALK);
                let heading = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                self.walk_on(heading, rng);
                return;
            }
            Ask::Sit { at, height } if visitor.reach <= 0.0 => {
                bird.stand_under(true);
                let beside = Self::beside(at);
                if self.shuffle_to(bird, beside, visitor.stand.y, dt) {
                    visitor.seat = Some((at, height));
                }
                gaze.glance_at(self.look_at(at, bird), 0.5);
                return;
            }
            _ => {}
        }
        if !visitor.at_counter {
            gaze.glance_at(self.look_at(player, bird), 0.5);
            let away = Vec2::new(self.x, self.z).distance(visitor.stand);
            if away < CLOSE_WITHIN {
                bird.stand_under(true);
            }
            let ahead = (visitor.stand.x - self.x) * self.heading;
            // It comes up to the counter straight from the street, so it
            // stands facing it without turning on the spot.
            let front = visitor.stand - Vec2::Y * STEP_UP;
            let here = Vec2::new(self.x, self.z);
            if here.y > front.y {
                // Already at the counter, from a stool.
                self.shuffle_to(bird, visitor.stand, visitor.stand.y, dt);
            } else if away < SHUT_BY && !bird.has_umbrella_shut() {
                bird.speed = (bird.speed - ACCELERATION * dt).max(0.0);
            } else if ahead > TURN_IN {
                // Aiming past where it turns, so it does not slow down.
                let along = Vec2::new(visitor.stand.x, self.lane);
                self.walk_to(bird, along, bird.pace, dt);
            } else if here.y < front.y {
                let past = front + (front - here).normalize_or_zero();
                self.walk_to(bird, past, bird.pace, dt);
            } else {
                self.walk_to(bird, visitor.stand, bird.pace * STEP_UP_PACE, dt);
            }
            return;
        }
        bird.stand_under(true);
        bird.speed = 0.0;
        self.yaw = turned(self.yaw, 0.0, TURN_RATE * dt);
        self.gesture(visitor, bird, voice, dt);
        self.look_around(visitor, bird, gaze, &counter, dt, rng);
        bird.lean_in(visitor.lean, dt);
        let showing = matches!(visitor.ask, Ask::Call | Ask::HoldOut { .. });
        bird.present(if showing { PRESENT } else { 0.0 }, dt);
    }

    /// At the counter, it looks at the player with one eye, and in between
    /// at what it carries, at the player's work, or off down the street.
    /// It shifts about while it waits.
    fn look_around(
        &self,
        visitor: &mut Visitor,
        bird: &Bird,
        gaze: &mut Gaze,
        counter: &CounterLayout,
        dt: f32,
        rng: &mut impl Rng,
    ) {
        visitor.look -= dt;
        if visitor.look <= 0.0 {
            visitor.at_player = !visitor.at_player;
            visitor.look = if visitor.at_player {
                rng.random_range(AT_PLAYER)
            } else {
                rng.random_range(ELSEWHERE)
            };
            let wanting = matches!(visitor.ask, Ask::Call | Ask::HoldOut { .. });
            visitor.gaze = match rng.random_range(0.0..1.0) {
                x if x < 0.5 && visitor.carrying.is_some() => Glance::Carried,
                x if x < 0.75 || wanting => Glance::Work,
                _ => Glance::Street(if rng.random_bool(0.5) { 1.0 } else { -1.0 }),
            };
            if matches!(visitor.ask, Ask::Wait) {
                visitor.lean = rng.random_range(FIDGET);
            }
        }
        visitor.lean = match visitor.ask {
            Ask::HoldOut { .. } => LEAN_OVER,
            Ask::Call => LEAN_CALL,
            Ask::Wait if visitor.lean > FIDGET.end => 0.0,
            _ => visitor.lean,
        };
        let player = counter.seated_view().translation;
        let target = if visitor.at_player {
            Some(player)
        } else {
            match visitor.gaze {
                Glance::Carried => bird.held().map(|held| held.translation),
                Glance::Work => Some(counter.mat.top_spot(Vec2::ZERO)),
                Glance::Street(side) => {
                    gaze.glance_at(Vec2::new(side * 1.2, 0.05), 0.5);
                    None
                }
            }
        };
        if let Some(target) = target {
            gaze.eye_at(self.look_at(target, bird), 0.5);
        }
    }

    /// Steps over to `to` at the counter, facing it: back from the counter
    /// at `line` first, along, and in again, so it keeps clear of the
    /// counter and the stools. Returns whether it is there.
    fn shuffle_to(&mut self, bird: &mut Bird, to: Vec2, line: f32, dt: f32) -> bool {
        let here = Vec2::new(self.x, self.z);
        let back = line - STEP_BACK;
        let goal = if (here.x - to.x).abs() < 0.05 {
            to
        } else if here.y > back + 0.03 {
            Vec2::new(here.x, back)
        } else {
            Vec2::new(to.x, back)
        };
        let way = goal - here;
        let distance = way.length();
        let target = SHUFFLE_PACE * (distance / SLOW_DOWN).min(1.0);
        let change = ACCELERATION * dt;
        bird.speed += (target - bird.speed).clamp(-change, change);
        let step = way.normalize_or_zero() * (bird.speed * dt).min(distance);
        self.x += step.x;
        self.z += step.y;
        self.yaw = turned(self.yaw, 0.0, TURN_RATE * dt);
        goal == to && distance < 0.04
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
                let bob = BOB * (into * BOB_RATE * TAU).sin();
                let up = EYES * bird.size() + SHOW_AT.y + bob;
                let show = root.transform_point(SHOW_AT.with_y(up));
                (show, into < SHOW_FOR)
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
        gaze.eye_at(self.look_at(target, bird), look);
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
