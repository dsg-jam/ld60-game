mod kit;
mod pose;

use std::f32::consts::TAU;
use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

pub(super) use self::kit::Kit;
use super::super::weather::Weather;
use super::gaze::Gaze;
use super::plumage::{Coat, PlumageMaterial};
use super::shadows::Silhouette;
use super::sound::{Footfall, Footfalls, Shakes, Voice};
use super::umbrella::{Canopy, Umbrella};
use super::{Figures, Pick, SITTER_TALK, WALKER_TALK, Whims};

const HIP_HEIGHT: f32 = 1.05;
const HIP_WIDTH: f32 = 0.2;
const THIGH: f32 = 0.06;
const SHIN: f32 = 0.035;
const HULL: Vec3 = Vec3::new(0.42, 0.46, 0.62);
/// Height of the middle of the hull above the hips.
const HULL_ABOVE: f32 = 0.18;
const NECK_BASE: Vec3 = Vec3::new(0.0, 0.12, 0.24);
const HEAD: Vec3 = Vec3::new(0.0, 0.42, 0.34);
const NECK: f32 = 0.07;
const SHOULDER: Vec3 = Vec3::new(0.17, 0.06, 0.18);
const ARM: f32 = 0.03;
/// Where the tail leaves the hull, as a share of its size.
const RUMP: Vec3 = Vec3::new(0.0, 0.1, -0.46);
/// Matches the skull in [`Figures::spawn_head`].
const SKULL: Vec3 = Vec3::new(0.2, 0.18, 0.28);
const FEELER_CHANCE: f64 = 0.25;
const UMBRELLA_CHANCE: f64 = 0.55;
/// Room an open umbrella takes. Less than its radius, as umbrellas tilt
/// away from each other.
const UMBRELLA_ROOM: f32 = 0.42;
/// Room for the swinging arms beside the hull.
const ELBOW_ROOM: f32 = 0.08;
/// The umbrellas open at this much rain.
const RAINY: f32 = 0.25;
/// Seconds to soak through in the rain, and to dry again.
const SOAKING: f32 = 40.0;
const DRYING: f32 = 240.0;
/// A wet local standing still shakes the water off now and then, this many
/// times a minute, for this many seconds.
const SHAKES: f32 = 2.0;
const SHAKE_TIME: f32 = 0.9;
/// Wetter than this, it may shake. A shake throws off this share of the
/// water.
const SHAKE_WET: f32 = 0.3;
const SHAKEN_OFF: f32 = 0.4;
const PACE: Range<f32> = 1.0..1.4;

/// A tall local on long legs, a bird of sorts: a round, feathered hull, a
/// head on a neck in front and a tail of plumes. Its local +Z is its front.
#[derive(Component, Debug)]
pub(super) struct Strider {
    size: f32,
    /// The size of the hull.
    hull: Vec3,
    posture: Posture,
    rig: Rig,
    coat: Coat,
    /// Steps taken, in cycles of both feet.
    phase: f32,
    /// Breaths taken, offset so they don't all breathe at once.
    breath: f32,
    /// Speed over the ground, set by what it is doing.
    pub(super) speed: f32,
    /// Its walking speed.
    pub(super) pace: f32,
    /// How far it hunches against the rain, from 0 to 1.
    pub(super) hunch: f32,
    /// Whether a roof keeps the rain off.
    sheltered: bool,
    /// How soaked its feathers are, from 0 to 1.
    wet: f32,
    /// Seconds left of shaking the water off.
    shaking: f32,
    tail: Tail,
    umbrella: Option<Umbrella>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Posture {
    Walking,
    /// Sitting on a seat this high, its feet forward on the ground.
    Seated(f32),
}

/// What a strider is and where it starts.
pub(super) struct Birth {
    pub(super) at: Transform,
    pub(super) posture: Posture,
    /// Whether it might carry an umbrella.
    pub(super) may_carry: bool,
    /// Whether a roof keeps the rain off where it stays.
    pub(super) sheltered: bool,
    pub(super) coat: Coat,
}

/// The tail swings after the hull on a spring.
#[derive(Debug, Default, Clone, Copy)]
struct Tail {
    /// Pitch up and swing to the side, in radians, and how fast they change.
    angle: Vec2,
    speed: Vec2,
    /// Where the hull was last frame, to feel it move.
    last: Option<Vec3>,
}

/// The moving parts.
#[derive(Debug)]
struct Rig {
    hull: Entity,
    plumage: Entity,
    head: Entity,
    tail: Entity,
    thighs: [Entity; 2],
    cuffs: [Entity; 2],
    shins: [Entity; 2],
    feet: [Entity; 2],
    arms: [Entity; 2],
    hands: [Entity; 2],
    sleeves: [Entity; 2],
}

impl Strider {
    pub(super) fn spawn(
        commands: &mut Commands,
        figures: &Figures,
        rng: &mut impl Rng,
        birth: Birth,
    ) -> Entity {
        let Birth {
            at,
            posture,
            may_carry,
            sheltered,
            coat,
        } = birth;
        let size = rng.random_range(0.85..1.15f32);
        let girth = Vec3::new(
            rng.random_range(0.85..1.25),
            rng.random_range(0.9..1.15),
            1.0,
        );
        let hull_size = HULL * girth * size;
        let colors = figures.colors(rng).with_face(&coat.down);
        let carries = may_carry && rng.random_bool(UMBRELLA_CHANCE);
        let kit = &figures.striders;
        let uniform = Transform::from_scale(Vec3::splat(size));
        let mut root = commands.spawn((
            Name::new("strider"),
            at,
            Visibility::default(),
            Gaze::default(),
            Silhouette::default(),
        ));
        let mut rig = None;
        root.with_children(|parent| {
            let mut hull = parent.spawn((Transform::default(), Visibility::default()));
            let mut head = Entity::PLACEHOLDER;
            let mut plumage = Entity::PLACEHOLDER;
            let mut tail = Entity::PLACEHOLDER;
            let mut arms = [Entity::PLACEHOLDER; 2];
            let mut hands = [Entity::PLACEHOLDER; 2];
            let mut sleeves = [Entity::PLACEHOLDER; 2];
            hull.with_children(|hull| {
                hull.spawn(figures.ball(&coat.down, Transform::from_scale(hull_size * 0.94)));
                plumage = hull
                    .spawn(coat.bundle(kit.bodies.pick(rng), Transform::from_scale(hull_size)))
                    .id();
                hull.spawn(coat.bundle(&kit.neck, uniform));
                tail = hull
                    .spawn(coat.bundle(
                        kit.tails.pick(rng),
                        Transform::from_translation(RUMP * hull_size),
                    ))
                    .id();
                head = hull
                    .spawn((
                        Transform::from_translation(HEAD * size),
                        Visibility::default(),
                    ))
                    .with_children(|head| {
                        let feelers = rng.random_bool(FEELER_CHANCE);
                        figures.spawn_head(head, &colors, size, feelers, rng);
                        head.spawn(coat.bundle(kit.crests.pick(rng), uniform));
                    })
                    .id();
                arms = [(); 2].map(|()| hull.spawn(figures.rod(&colors.skin, default())).id());
                hands = [(); 2].map(|()| hull.spawn(figures.ball(&colors.skin, default())).id());
                sleeves = [(); 2].map(|()| hull.spawn(coat.bundle(&kit.sleeve, default())).id());
            });
            let hull = hull.id();
            let thighs = [(); 2].map(|()| parent.spawn(figures.rod(&coat.down, default())).id());
            let cuffs = [(); 2].map(|()| parent.spawn(coat.bundle(&kit.cuff, default())).id());
            let shins = [(); 2].map(|()| parent.spawn(figures.rod(&colors.skin, default())).id());
            let feet = [(); 2].map(|()| {
                parent
                    .spawn((
                        Mesh3d(kit.toes.clone()),
                        MeshMaterial3d(colors.skin.clone()),
                        Transform::default(),
                    ))
                    .id()
            });
            let umbrella = carries.then(|| Umbrella::spawn(parent, &figures.umbrellas, rng, size));
            rig = Some((
                Rig {
                    hull,
                    plumage,
                    head,
                    tail,
                    thighs,
                    cuffs,
                    shins,
                    feet,
                    arms,
                    hands,
                    sleeves,
                },
                umbrella,
            ));
        });
        let Some((rig, umbrella)) = rig else {
            unreachable!("with_children runs at once");
        };
        let talk = match posture {
            Posture::Walking => WALKER_TALK,
            Posture::Seated(_) => SITTER_TALK,
        };
        root.insert((
            Voice::new(size, talk, rng),
            Self {
                size,
                hull: hull_size,
                posture,
                rig,
                coat,
                phase: rng.random(),
                breath: rng.random_range(0.0..TAU),
                speed: 0.0,
                pace: rng.random_range(PACE) * size,
                hunch: 0.0,
                sheltered,
                wet: 0.0,
                shaking: 0.0,
                tail: Tail::default(),
                umbrella,
            },
        ));
        root.id()
    }

    pub(super) fn has_umbrella_open(&self) -> bool {
        self.umbrella.as_ref().is_some_and(Umbrella::is_open)
    }

    /// Tilts its umbrella away from `away`, a direction in its own space, as
    /// far as `crowded` from 0 to 1.
    pub(super) fn lean_umbrella(&mut self, away: Vec2, crowded: f32, dt: f32) {
        if let Some(umbrella) = &mut self.umbrella {
            umbrella.lean_away(away, crowded, dt);
        }
    }

    /// How far others stay from its middle, umbrella included.
    pub(super) fn radius(&self) -> f32 {
        if self.has_umbrella_open() {
            UMBRELLA_ROOM * self.size
        } else {
            self.hull.x / 2.0 + ELBOW_ROOM
        }
    }

    /// How much of `rain` falls on it, from 0 to 1.
    pub(super) fn rained_on(&self, rain: f32) -> f32 {
        if self.sheltered {
            return 0.0;
        }
        let open = self.umbrella.as_ref().map_or(0.0, Umbrella::openness);
        rain * (1.0 - open)
    }

    /// Opens the umbrellas when it rains and closes them when it stops,
    /// each a little later than the others.
    pub(super) fn open_umbrellas(
        time: Res<Time>,
        weather: Res<Weather>,
        mut whims: ResMut<Whims>,
        mut striders: Query<&mut Strider>,
    ) {
        let dt = time.delta_secs();
        let raining = weather.rain >= RAINY;
        for mut strider in &mut striders {
            if let Some(umbrella) = &mut strider.umbrella {
                umbrella.follow_rain(raining, dt, &mut whims.0);
            }
        }
    }

    /// Soaks the feathers in the rain and dries them after. Wet locals
    /// standing still now and then shake the water off.
    pub(super) fn soak(
        time: Res<Time>,
        weather: Res<Weather>,
        mut whims: ResMut<Whims>,
        mut shakes: ResMut<Shakes>,
        mut plumages: ResMut<Assets<PlumageMaterial>>,
        mut striders: Query<(&mut Strider, &Transform)>,
    ) {
        let dt = time.delta_secs();
        let rng = &mut whims.0;
        for (mut strider, transform) in &mut striders {
            let rain = strider.rained_on(weather.rain);
            let rate = if rain > strider.wet {
                1.0 / SOAKING
            } else {
                1.0 / DRYING
            };
            strider.wet += (rain - strider.wet) * (rate * dt).min(1.0);
            strider.shaking = (strider.shaking - dt).max(0.0);
            let shake = strider.speed < 0.2
                && strider.shaking <= 0.0
                && strider.wet > SHAKE_WET
                && rng.random_bool(f64::from((SHAKES / 60.0 * dt).clamp(0.0, 1.0)));
            if shake {
                strider.shaking = SHAKE_TIME;
                strider.wet *= 1.0 - SHAKEN_OFF;
                shakes
                    .0
                    .push(transform.transform_point(Vec3::Y * HIP_HEIGHT * strider.size));
            }
            let wet = strider.wet;
            strider.coat.soak(wet, &mut plumages);
        }
    }

    pub(super) fn pose(
        time: Res<Time>,
        mut footfalls: ResMut<Footfalls>,
        mut striders: Query<(&mut Strider, &Gaze, &Transform, &mut Silhouette)>,
        mut parts: Query<&mut Transform, Without<Strider>>,
        mut visibilities: Query<&mut Visibility>,
        mut canopies: Query<&mut Canopy>,
    ) {
        let dt = time.delta_secs();
        for (mut strider, gaze, body, mut silhouette) in &mut striders {
            let before = strider.phase;
            strider.step(dt, body);
            let pose = strider.posed(gaze);
            let gait = strider.gait();
            for (index, offset) in [0.0, 0.5].into_iter().enumerate() {
                // A foot lands as its cycle starts over.
                let landed = (strider.phase + offset).fract() < (before + offset).fract();
                if landed && gait > pose::STEPPING {
                    footfalls.0.push(Footfall {
                        at: body.transform_point(pose.feet[index].translation),
                        level: gait * strider.size,
                        indoors: false,
                    });
                }
            }
            *silhouette = pose.silhouette.moved(body);
            let rig = &strider.rig;
            let mut set = |entity: Entity, transform: Transform| {
                if let Ok(mut part) = parts.get_mut(entity) {
                    *part = transform;
                }
            };
            set(rig.hull, pose.hull);
            set(rig.plumage, pose.plumage);
            set(rig.head, pose.head);
            set(rig.tail, pose.tail);
            for side in 0..2 {
                set(rig.thighs[side], pose.thighs[side]);
                set(rig.cuffs[side], pose.cuffs[side]);
                set(rig.shins[side], pose.shins[side]);
                set(rig.feet[side], pose.feet[side]);
                set(rig.arms[side], pose.arms[side]);
                set(rig.hands[side], pose.hands[side]);
                set(rig.sleeves[side], pose.sleeves[side]);
            }
            if let (Some(umbrella), Some(placed)) = (&strider.umbrella, pose.umbrella) {
                let mut show = |entity: Entity, shown: bool| {
                    if let Ok(mut visibility) = visibilities.get_mut(entity) {
                        visibility.set_if_neq(if shown {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        });
                    }
                };
                umbrella.pose(placed, strider.size, &mut set, &mut show);
                if let Ok(mut canopy) = canopies.get_mut(umbrella.canopy()) {
                    canopy.open = umbrella.openness();
                }
            }
        }
    }
}
