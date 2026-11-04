use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::super::weather::Weather;
use super::gaze::{self, Gaze};
use super::limb::{Limb, rod};
use super::shadows::{Blob, Bone, Silhouette};
use super::sound::{Footfall, Footfalls, Voice};
use super::umbrella::{self, Canopy, Umbrella};
use super::{Figures, SITTER_TALK, WALKER_TALK, Whims};

const HIP_HEIGHT: f32 = 1.05;
const LEG: Limb = Limb {
    upper: 0.6,
    lower: 0.64,
};
const HIP_WIDTH: f32 = 0.2;
const THIGH: f32 = 0.06;
const SHIN: f32 = 0.04;
const FOOT: Vec3 = Vec3::new(0.09, 0.04, 0.17);
/// The knees bend back, like a bird's.
const KNEES: Vec3 = Vec3::NEG_Z;
/// Seated, the knees rise up in front.
const SEATED_KNEES: Vec3 = Vec3::new(0.0, 1.0, 0.4);
const HULL: Vec3 = Vec3::new(0.42, 0.46, 0.62);
/// Height of the middle of the hull above the hips.
const HULL_ABOVE: f32 = 0.18;
/// Where the hips are under the hull, in its space.
const HIPS: Vec3 = Vec3::new(HIP_WIDTH / 2.0, -HULL_ABOVE, -0.04);
const NECK_BASE: Vec3 = Vec3::new(0.0, 0.12, 0.24);
const HEAD: Vec3 = Vec3::new(0.0, 0.42, 0.34);
/// Matches the skull in [`Figures::spawn_head`].
const SKULL: Vec3 = Vec3::new(0.2, 0.18, 0.28);
const NECK: f32 = 0.07;
const SHOULDER: Vec3 = Vec3::new(0.17, 0.06, 0.18);
const ARM: f32 = 0.035;
/// Where the hands hang from the shoulders, in the hull's space.
const HANG: Vec3 = Vec3::new(0.03, -0.34, 0.12);
/// Seated, the hands rest in the lap.
const LAP: Vec3 = Vec3::new(0.12, -0.12, 0.34);
const ARM_SWING: f32 = 0.12;
/// The hand holding an open umbrella, in the hull's space: in front of the
/// chest, to the side of the head.
const HOLD: Vec3 = Vec3::new(0.2, 0.2, 0.3);
const UMBRELLA_CHANCE: f64 = 0.55;
/// Room an open umbrella takes. Less than its radius, as umbrellas tilt
/// away from each other.
const UMBRELLA_ROOM: f32 = 0.42;
/// With an open umbrella, the head turns only this far towards it.
const UMBRELLA_GLANCE: f32 = 0.45;
/// Room for the swinging arms beside the hull.
const ELBOW_ROOM: f32 = 0.08;
/// The umbrellas open at this much rain.
const RAINY: f32 = 0.25;

/// Share of a step's cycle that a foot stands on the ground.
const STANCE: f32 = 0.62;
const STRIDE: f32 = 0.62;
const LIFT: f32 = 0.09;
const BOB: f32 = 0.025;
const SWAY: f32 = 0.025;
const LEAN: f32 = 0.12;
const HUNCH_LEAN: f32 = 0.35;
const HUNCH_DROP: f32 = 0.08;
/// Feet are heard landing above this much of the walking speed.
const STEPPING: f32 = 0.2;
/// Breaths per second, and how far the hull rises with one.
const BREATH_RATE: f32 = 0.3;
const BREATH: f32 = 0.006;

/// A tall local on long legs, with a round hull and a head on a neck in
/// front. Its local +Z is its front.
#[derive(Component, Debug)]
pub(super) struct Strider {
    size: f32,
    /// The size of the hull.
    hull: Vec3,
    posture: Posture,
    rig: Rig,
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
    umbrella: Option<Umbrella>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Posture {
    Walking,
    /// Sitting on a seat this high, its feet forward on the ground.
    Seated(f32),
}

/// The moving parts.
#[derive(Debug)]
struct Rig {
    hull: Entity,
    head: Entity,
    thighs: [Entity; 2],
    shins: [Entity; 2],
    feet: [Entity; 2],
    arms: [Entity; 2],
}

impl Strider {
    /// A new strider at `at`, sometimes with an umbrella if `may_carry`.
    pub(super) fn spawn(
        commands: &mut Commands,
        figures: &Figures,
        rng: &mut impl Rng,
        at: Transform,
        posture: Posture,
        may_carry: bool,
    ) -> Entity {
        let size = rng.random_range(0.85..1.15f32);
        let girth = Vec3::new(
            rng.random_range(0.85..1.25),
            rng.random_range(0.9..1.15),
            1.0,
        );
        let colors = figures.colors(rng);
        let carries = may_carry && rng.random_bool(UMBRELLA_CHANCE);
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
            let mut arms = [Entity::PLACEHOLDER; 2];
            hull.with_children(|hull| {
                hull.spawn(figures.ball(
                    &colors.cloth,
                    Transform::default().with_scale(HULL * girth * size),
                ));
                hull.spawn(figures.rod(
                    &colors.skin,
                    rod(NECK_BASE * size, HEAD * size, NECK * size),
                ));
                head = hull
                    .spawn((
                        Transform::from_translation(HEAD * size),
                        Visibility::default(),
                    ))
                    .with_children(|head| figures.spawn_head(head, &colors, size, rng))
                    .id();
                arms = [(); 2].map(|()| hull.spawn(figures.rod(&colors.skin, default())).id());
            });
            let hull = hull.id();
            let thighs = [(); 2].map(|()| parent.spawn(figures.rod(&colors.skin, default())).id());
            let shins = [(); 2].map(|()| parent.spawn(figures.rod(&colors.skin, default())).id());
            let feet = [(); 2].map(|()| parent.spawn(figures.ball(&colors.skin, default())).id());
            let umbrella = carries.then(|| Umbrella::spawn(parent, &figures.umbrellas, rng, size));
            rig = Some((
                Rig {
                    hull,
                    head,
                    thighs,
                    shins,
                    feet,
                    arms,
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
                hull: HULL * girth * size,
                posture,
                rig,
                phase: rng.random(),
                breath: rng.random_range(0.0..TAU),
                speed: 0.0,
                pace: rng.random_range(1.0..1.4) * size,
                hunch: 0.0,
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
            strider.breath += dt * BREATH_RATE * TAU;
            let before = strider.phase;
            strider.phase += strider.speed * dt * STANCE / (STRIDE * strider.size);
            strider.phase = strider.phase.fract();
            let pose = strider.posed(gaze);
            let gait = strider.gait();
            for (index, offset) in [0.0, 0.5].into_iter().enumerate() {
                // A foot lands as its cycle starts over.
                let landed = (strider.phase + offset).fract() < (before + offset).fract();
                if landed && gait > STEPPING {
                    let at = body.transform_point(pose.feet[index].translation);
                    footfalls.0.push(Footfall {
                        at,
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
            set(rig.head, pose.head);
            for side in 0..2 {
                set(rig.thighs[side], pose.thighs[side]);
                set(rig.shins[side], pose.shins[side]);
                set(rig.feet[side], pose.feet[side]);
                set(rig.arms[side], pose.arms[side]);
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

    /// Where every part goes now.
    fn posed(&self, gaze: &Gaze) -> Pose {
        let size = self.size;
        let breath = BREATH * size * self.breath.sin();
        let (hull, feet, knees, lean) = match self.posture {
            Posture::Walking => self.walking(breath),
            Posture::Seated(height) => {
                let hull = Vec3::new(0.0, height + HULL_ABOVE * size + breath, 0.0);
                let feet = [-1.0, 1.0].map(|side| Vec3::new(side * 0.17, 0.0, 0.28) * size);
                (Transform::from_translation(hull), feet, SEATED_KNEES, 0.0)
            }
        };
        let open = self.umbrella.as_ref().map_or(0.0, Umbrella::openness);
        let glance = gaze.turn.x.min(gaze::YAW.lerp(UMBRELLA_GLANCE, open));
        let mut pose = Pose {
            hull,
            head: Transform::from_translation(HEAD * size).with_rotation(
                Quat::from_rotation_y(glance)
                    * Quat::from_rotation_x(gaze.turn.y + self.hunch * 0.3 - lean * 0.8),
            ),
            thighs: [Transform::default(); 2],
            shins: [Transform::default(); 2],
            feet: [Transform::default(); 2],
            arms: [Transform::default(); 2],
            umbrella: None,
            silhouette: Silhouette::default(),
        };
        let skull = hull * pose.head * Transform::from_xyz(0.0, 0.0, 0.04 * size);
        let shadow = &mut pose.silhouette;
        shadow.blobs[0] = Blob::new(&hull, self.hull);
        shadow.blobs[1] = Blob::new(&skull, SKULL * size);
        shadow.height = skull.translation.y + SKULL.y * size / 2.0;
        shadow.bones[4] = Bone {
            from: hull.transform_point(NECK_BASE * size),
            to: hull.transform_point(HEAD * size),
            radius: NECK * size / 2.0,
        };
        for (index, side) in [-1.0f32, 1.0].into_iter().enumerate() {
            let flip = Vec3::new(side, 1.0, 1.0);
            let hip = hull.transform_point(HIPS * flip * size);
            let ankle = feet[index] + Vec3::Y * FOOT.y * size;
            let (knee, ankle) = (LEG * size).reach(hip, ankle, knees);
            pose.thighs[index] = rod(hip, knee, THIGH * size);
            pose.shins[index] = rod(knee, ankle, SHIN * size);
            pose.silhouette.bones[index] = Bone {
                from: hip,
                to: knee,
                radius: THIGH * size / 2.0,
            };
            pose.silhouette.bones[2 + index] = Bone {
                from: knee,
                to: ankle,
                radius: SHIN * size / 2.0,
            };
            let lifted = (ankle.y - FOOT.y * size).max(0.0);
            pose.feet[index] = Transform::from_translation(
                ankle + Vec3::new(0.0, -FOOT.y * size / 2.0, FOOT.z * size * 0.3),
            )
            .with_rotation(Quat::from_rotation_x(-lifted * 3.0))
            .with_scale(FOOT * size);

            let shoulder = SHOULDER * flip * size;
            let swing = (self.phase + index as f32 * 0.5) * TAU;
            let swing = ARM_SWING * self.gait() * swing.sin();
            let mut hand = match self.posture {
                Posture::Walking => shoulder + (HANG * flip + Vec3::Z * swing) * size,
                Posture::Seated(_) => LAP * flip * size,
            };
            if let Some(umbrella) = self.umbrella.as_ref().filter(|_| index == 1) {
                hand = umbrella.hand(hand, HOLD * size);
                let placed = umbrella.place(hull.transform_point(hand), swing * 2.0);
                if let Some(canopy) = umbrella.shadow(&placed, size) {
                    pose.silhouette.blobs[2] = canopy;
                    pose.silhouette.height = placed.translation.y + umbrella::SHAFT * size;
                }
                pose.umbrella = Some(placed);
            }
            pose.arms[index] = rod(shoulder, hand, ARM * size);
            pose.silhouette.bones[5 + index] = Bone {
                from: hull.transform_point(shoulder),
                to: hull.transform_point(hand),
                radius: ARM * size / 2.0,
            };
        }
        pose
    }

    /// How far into its stride it is, from standing at 0 to walking at 1.
    fn gait(&self) -> f32 {
        (self.speed / self.pace).clamp(0.0, 1.0)
    }

    /// The hull, the spots of the feet, the bend of the knees and the lean
    /// of the hull.
    fn walking(&self, breath: f32) -> (Transform, [Vec3; 2], Vec3, f32) {
        let size = self.size;
        let gait = self.gait();
        let stride = STRIDE * size;
        let bob = BOB * size * gait * (2.0 * TAU * self.phase).cos();
        let sway = SWAY * size * gait * (TAU * self.phase).sin();
        let height = (HIP_HEIGHT - HUNCH_DROP * self.hunch) * size + HULL_ABOVE * size;
        let lean = LEAN * gait + HUNCH_LEAN * self.hunch;
        let hull = Transform::from_xyz(sway, height + bob + breath, 0.0)
            .with_rotation(Quat::from_rotation_x(lean) * Quat::from_rotation_z(-sway * 2.0));
        let feet = [0.0, 0.5].map(|offset| {
            let cycle = (self.phase + offset).fract();
            if cycle < STANCE {
                Vec3::Z * stride * (0.5 - cycle / STANCE)
            } else {
                let t = (cycle - STANCE) / (1.0 - STANCE);
                let ahead = t * t * (3.0 - 2.0 * t);
                Vec3::new(
                    0.0,
                    (PI * t).sin() * LIFT * size * gait,
                    stride * (ahead - 0.5),
                )
            }
        });
        let feet = [
            feet[0] + Vec3::X * -HIP_WIDTH * 0.6 * size,
            feet[1] + Vec3::X * HIP_WIDTH * 0.6 * size,
        ];
        (hull, feet, KNEES, lean)
    }
}

/// The parts of a [`Strider`], in its own space except for the head, which
/// is in the hull's space like the arms.
struct Pose {
    hull: Transform,
    head: Transform,
    thighs: [Transform; 2],
    shins: [Transform; 2],
    feet: [Transform; 2],
    arms: [Transform; 2],
    /// The umbrella in its hand, in its own space.
    umbrella: Option<Transform>,
    /// In its own space.
    silhouette: Silhouette,
}
