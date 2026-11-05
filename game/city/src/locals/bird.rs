mod dress;
mod head;
mod model;
mod pose;
mod skeleton;

use std::f32::consts::TAU;
use std::ops::Range;

use bevy::{
    camera::visibility::DynamicSkinnedMeshBounds, light::NotShadowCaster,
    mesh::skinning::SkinnedMesh, prelude::*,
};
use rand::{Rng, RngExt};

pub(super) use self::dress::Dress;
pub(super) use self::model::Kit;
pub(super) use self::pose::Work;
use self::skeleton::{Bone, Side};
use super::super::weather::Weather;
use super::gaze::Gaze;
use super::keeper::Keeper;
use super::plumage::{Coat, PlumageMaterial};
use super::shadows::Silhouette;
use super::sound::{Footfall, Footfalls, Shakes, Voice};
use super::umbrella::{Canopy, Umbrella};
use super::{Figures, KEEPER_TALK, Pick, WALKER_TALK, Whims};

/// The model is about 1.3 m tall, so most birds stand about 1.6 m.
const SIZE: Range<f32> = 1.15..1.35;
/// Umbrellas for birds are a little smaller than ours.
const UMBRELLA_SIZE: f32 = 0.85;
const PACE: Range<f32> = 0.55..0.9;
/// Some take longer steps than others, at the same size.
const STRIDES: Range<f32> = 0.85..1.15;
const UMBRELLA_CHANCE: f64 = 0.55;
/// Room an open umbrella takes. Less than its radius, as umbrellas tilt
/// away from each other.
const UMBRELLA_ROOM: f32 = 0.4;
/// Room for the arms beside the hull.
const ELBOW_ROOM: f32 = 0.06;
/// The umbrellas open at this much rain.
const RAINY: f32 = 0.25;
/// Seconds to soak through in the rain, and to dry again.
const SOAKING: f32 = 40.0;
const DRYING: f32 = 240.0;
/// A wet bird standing still shakes the water off now and then, this many
/// times a minute, for this many seconds.
const SHAKES: f32 = 2.0;
const SHAKE_TIME: f32 = 0.9;
/// Wetter than this, it may shake. A shake throws off this share of the
/// water.
const SHAKE_WET: f32 = 0.3;
const SHAKEN_OFF: f32 = 0.4;
/// Seconds between blinks, and of a blink.
const BLINKS: Range<f32> = 2.0..6.0;
const BLINK: f32 = 0.13;

/// A local: an upright heron of sorts, with a long neck and beak, legs
/// that bend back, and folded wings with hands at their wrists. Its local
/// +Z is its front.
///
/// It walks the sidewalks, sits on the stools of the stalls, or keeps a
/// stall from behind its counter, all with the same body.
#[derive(Component, Debug)]
pub(super) struct Bird {
    size: f32,
    /// How much wider and taller than usual its hull is.
    girth: Vec3,
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
    /// How much longer its steps are than usual.
    stride: f32,
    /// How far it hunches against the rain, from 0 to 1.
    pub(super) hunch: f32,
    /// Whether a roof keeps the rain off.
    sheltered: bool,
    /// How far it has sat down, from standing at 0 to seated at 1, and
    /// the height of the seat in meters.
    sitting: f32,
    seat: f32,
    /// How soaked its feathers are, from 0 to 1.
    wet: f32,
    /// Seconds left of shaking the water off.
    shaking: f32,
    /// Seconds until the next blink, below zero while blinking.
    blink: f32,
    tail: pose::Swing,
    scarf: pose::Swing,
    umbrella: Option<Umbrella>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Posture {
    /// Out and about: walking, standing or sitting down.
    Out,
    /// Standing behind a counter, its hands on it.
    Keeping,
}

/// What a bird is and where it starts.
pub(super) struct Birth {
    pub(super) at: Transform,
    pub(super) posture: Posture,
    pub(super) carry: Carry,
    pub(super) dress: Dress,
    /// Whether a roof keeps the rain off where it stays.
    pub(super) sheltered: bool,
    pub(super) coat: Coat,
}

/// Whether a bird carries an umbrella.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Carry {
    Never,
    Maybe,
    #[cfg_attr(
        not(feature = "debug_port"),
        expect(dead_code, reason = "only lined up locals always carry one")
    )]
    Always,
}

/// The entities that move.
#[derive(Debug)]
struct Rig {
    /// In the order of [`Bone::joint`].
    joints: [Entity; Bone::COUNT],
    eyes: [Entity; 2],
}

impl Bird {
    pub(super) fn spawn(
        commands: &mut Commands,
        figures: &Figures,
        rng: &mut impl Rng,
        birth: Birth,
    ) -> Entity {
        let Birth {
            at,
            posture,
            carry,
            dress,
            sheltered,
            coat,
        } = birth;
        let size = rng.random_range(SIZE);
        let girth = Vec3::new(
            rng.random_range(0.9..1.15),
            rng.random_range(0.92..1.1),
            rng.random_range(0.95..1.08),
        );
        let carries = match carry {
            Carry::Never => false,
            Carry::Maybe => rng.random_bool(UMBRELLA_CHANCE),
            Carry::Always => true,
        };
        let kit = &figures.birds;
        let iris = kit.irises.pick(rng).clone();
        let mut root = commands.spawn((
            Name::new("local"),
            at,
            Visibility::default(),
            Gaze::default(),
            Silhouette::default(),
        ));
        let mut rig = None;
        root.with_children(|parent| {
            let mut body = parent.spawn((
                Transform::from_scale(Vec3::splat(size)),
                Visibility::default(),
            ));
            let mut joints = [Entity::PLACEHOLDER; Bone::COUNT];
            let mut eyes = [Entity::PLACEHOLDER; 2];
            let mut umbrella = None;
            body.with_children(|body| {
                for bone in Bone::all() {
                    let mut joint = body.spawn((bone.rest(), Visibility::default()));
                    if bone == Bone::Head {
                        joint.with_children(|head| {
                            eyes = Side::BOTH.map(|side| Self::spawn_eye(head, kit, &iris, side));
                        });
                    }
                    joints[bone.joint() as usize] = joint.id();
                }
                let skinned = SkinnedMesh {
                    inverse_bindposes: kit.bindposes.clone(),
                    joints: joints.to_vec(),
                };
                let plumages = if dress == Dress::Bare {
                    &kit.plumages
                } else {
                    &kit.plumages_under_cloth
                };
                body.spawn((
                    Mesh3d(plumages.pick(rng).clone()),
                    MeshMaterial3d(coat.feathers.clone()),
                    skinned.clone(),
                    DynamicSkinnedMeshBounds,
                    NotShadowCaster,
                ));
                body.spawn((
                    Mesh3d(kit.skin.clone()),
                    MeshMaterial3d(coat.skin.clone()),
                    skinned.clone(),
                    DynamicSkinnedMeshBounds,
                    NotShadowCaster,
                ));
                body.spawn((
                    Mesh3d(kit.bill.clone()),
                    MeshMaterial3d(coat.bill.clone()),
                    skinned.clone(),
                    DynamicSkinnedMeshBounds,
                    NotShadowCaster,
                ));
                let dress = match dress {
                    Dress::Bare => None,
                    Dress::Scarf => Some(kit.scarves.pick(rng)),
                    Dress::Cowl => Some(kit.cowls.pick(rng)),
                };
                if let Some(dress) = dress {
                    body.spawn((
                        Mesh3d(dress.clone()),
                        MeshMaterial3d(coat.feathers.clone()),
                        skinned,
                        DynamicSkinnedMeshBounds,
                        NotShadowCaster,
                    ));
                }
                umbrella =
                    carries.then(|| Umbrella::spawn(body, &figures.umbrellas, rng, UMBRELLA_SIZE));
            });
            rig = Some((Rig { joints, eyes }, umbrella));
        });
        let Some((rig, umbrella)) = rig else {
            unreachable!("with_children runs at once");
        };
        let talk = match posture {
            Posture::Out => WALKER_TALK,
            Posture::Keeping => KEEPER_TALK,
        };
        root.insert((
            Voice::new(size, talk, rng),
            Self {
                size,
                girth,
                posture,
                rig,
                coat,
                phase: rng.random(),
                breath: rng.random_range(0.0..TAU),
                speed: 0.0,
                pace: rng.random_range(PACE) * size,
                stride: rng.random_range(STRIDES),
                hunch: 0.0,
                sheltered,
                sitting: 0.0,
                seat: 0.0,
                wet: 0.0,
                shaking: 0.0,
                blink: rng.random_range(BLINKS),
                tail: pose::Swing::default(),
                scarf: pose::Swing::default(),
                umbrella,
            },
        ));
        root.id()
    }

    /// An eye on the head joint, looking out to its side and a little
    /// forward, with its pupil.
    fn spawn_eye(
        head: &mut ChildSpawnerCommands,
        kit: &Kit,
        iris: &Handle<StandardMaterial>,
        side: Side,
    ) -> Entity {
        head.spawn((
            Mesh3d(kit.eye.clone()),
            MeshMaterial3d(iris.clone()),
            Self::eye(side, 1.0),
            NotShadowCaster,
        ))
        .with_children(|eye| {
            eye.spawn((
                Mesh3d(kit.pupil.clone()),
                MeshMaterial3d(kit.pupil_black.clone()),
                Transform::from_translation(side.mirror(Kit::pupil_at()))
                    .with_scale(Vec3::new(0.45, 1.0, 1.0)),
                NotShadowCaster,
            ));
        })
        .id()
    }

    /// An eye on `side`, as open as `open` from 0 to 1, on the head joint.
    fn eye(side: Side, open: f32) -> Transform {
        Transform::from_translation(side.mirror(head::eye()))
            .with_rotation(Quat::from_rotation_y(0.4 * side.sign()))
            .with_scale(Vec3::new(1.0, open.max(0.08), 1.0))
    }

    pub(super) fn size(&self) -> f32 {
        self.size
    }

    /// Holds it still at `phase` of its steps, walking in place, or
    /// standing without one.
    #[cfg(feature = "debug_port")]
    pub(super) fn hold_step(&mut self, phase: Option<f32>) {
        self.speed = if phase.is_some() { self.pace } else { 0.0 };
        self.phase = phase.unwrap_or(0.0).rem_euclid(1.0);
    }

    /// Holds its umbrella, if it has one, `open` of the way through
    /// opening, or closing if not `opening`.
    #[cfg(feature = "debug_port")]
    pub(super) fn hold_umbrella(&mut self, open: f32, opening: bool) {
        if let Some(umbrella) = &mut self.umbrella {
            umbrella.hold_open(open, opening);
        }
    }

    /// Sits down on a seat `height` meters high, as far as `amount` from
    /// standing at 0 to seated at 1. A `covered` seat keeps the rain off
    /// once it sits.
    pub(super) fn sit(&mut self, height: f32, amount: f32, covered: bool) {
        self.seat = height;
        self.sitting = amount.clamp(0.0, 1.0);
        self.sheltered = covered && self.sitting > 0.5;
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
            (skeleton::HULL_RADII.x * self.girth.x + ELBOW_ROOM) * self.size
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
        mut birds: Query<&mut Bird>,
    ) {
        let dt = time.delta_secs();
        let raining = weather.rain >= RAINY;
        for mut bird in &mut birds {
            let open = raining && !bird.sheltered;
            if let Some(umbrella) = &mut bird.umbrella {
                umbrella.follow_rain(open, dt, &mut whims.0);
            }
        }
    }

    /// Soaks the feathers in the rain and dries them after. Wet birds
    /// standing still now and then shake the water off. Blinks.
    pub(super) fn live(
        time: Res<Time>,
        weather: Res<Weather>,
        mut whims: ResMut<Whims>,
        mut shakes: ResMut<Shakes>,
        mut plumages: ResMut<Assets<PlumageMaterial>>,
        mut birds: Query<(&mut Bird, &Transform)>,
    ) {
        let dt = time.delta_secs();
        let rng = &mut whims.0;
        for (mut bird, transform) in &mut birds {
            let rain = bird.rained_on(weather.rain);
            let rate = if rain > bird.wet {
                1.0 / SOAKING
            } else {
                1.0 / DRYING
            };
            bird.wet += (rain - bird.wet) * (rate * dt).min(1.0);
            bird.shaking = (bird.shaking - dt).max(0.0);
            let shake = bird.speed < 0.2
                && bird.shaking <= 0.0
                && bird.wet > SHAKE_WET
                && rng.random_bool(f64::from((SHAKES / 60.0 * dt).clamp(0.0, 1.0)));
            if shake {
                bird.shaking = SHAKE_TIME;
                bird.wet *= 1.0 - SHAKEN_OFF;
                shakes
                    .0
                    .push(transform.transform_point(Vec3::Y * skeleton::HULL.y * bird.size));
            }
            bird.blink -= dt;
            if bird.blink < -BLINK {
                bird.blink = rng.random_range(BLINKS);
            }
            let wet = bird.wet;
            bird.coat.soak(wet, &mut plumages);
        }
    }

    pub(super) fn pose(
        time: Res<Time>,
        mut footfalls: ResMut<Footfalls>,
        mut birds: Query<(
            &mut Bird,
            &Gaze,
            &Transform,
            &mut Silhouette,
            Option<&Keeper>,
        )>,
        mut parts: Query<&mut Transform, Without<Bird>>,
        mut visibilities: Query<&mut Visibility>,
        mut canopies: Query<&mut Canopy>,
    ) {
        let dt = time.delta_secs();
        for (mut bird, gaze, root, mut silhouette, keeper) in &mut birds {
            let body = *root * Transform::from_scale(Vec3::splat(bird.size));
            let work = keeper.map(|keeper| keeper.work(bird.size));
            let before = bird.phase;
            bird.step(dt, &body, work.as_ref());
            let pose = bird.posed(gaze, work.as_ref());
            let gait = bird.gait();
            for (index, offset) in [0.0, 0.5].into_iter().enumerate() {
                // A foot lands as its cycle starts over.
                let landed = (bird.phase + offset).fract() < (before + offset).fract();
                if landed && gait > pose::STEPPING && bird.posture == Posture::Out {
                    footfalls.0.push(Footfall {
                        at: body.transform_point(pose.feet[index]),
                        level: gait * bird.size,
                        indoors: false,
                    });
                }
            }
            *silhouette = pose.silhouette.moved(&body);
            let rig = &bird.rig;
            let mut set = |entity: Entity, transform: Transform| {
                if let Ok(mut part) = parts.get_mut(entity) {
                    *part = transform;
                }
            };
            for (joint, transform) in rig.joints.iter().zip(pose.joints) {
                set(*joint, transform);
            }
            let open = if bird.blink < 0.0 { 0.0 } else { 1.0 };
            for (eye, side) in rig.eyes.iter().zip(Side::BOTH) {
                set(*eye, Self::eye(side, open));
            }
            if let (Some(umbrella), Some(placed)) = (&bird.umbrella, pose.umbrella) {
                let mut show = |entity: Entity, shown: bool| {
                    if let Ok(mut visibility) = visibilities.get_mut(entity) {
                        visibility.set_if_neq(if shown {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        });
                    }
                };
                umbrella.pose(placed, UMBRELLA_SIZE, &mut set, &mut show);
                if let Ok(mut canopy) = canopies.get_mut(umbrella.canopy()) {
                    canopy.open = umbrella.openness();
                    canopy.shaken = umbrella.shaken();
                }
            }
        }
    }
}
