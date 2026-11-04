use std::f32::consts::{PI, TAU};

use bevy::prelude::*;

use super::super::gaze::{self, Gaze};
use super::super::limb::{Limb, rod};
use super::super::shadows::{Blob, Bone, Silhouette};
use super::super::umbrella::{self, Umbrella};
use super::{
    ARM, HEAD, HIP_HEIGHT, HIP_WIDTH, HULL_ABOVE, NECK, NECK_BASE, Posture, RUMP, SHAKE_TIME, SHIN,
    SHOULDER, SKULL, Strider, THIGH,
};

const LEG: Limb = Limb {
    upper: 0.6,
    lower: 0.64,
};
/// Height of the ankle above the sole.
const ANKLE: f32 = 0.04;
/// The knees bend back, like a bird's.
const KNEES: Vec3 = Vec3::NEG_Z;
/// Seated, the knees rise up in front.
const SEATED_KNEES: Vec3 = Vec3::new(0.0, 1.0, 0.4);
/// Where the hips are under the hull, in its space.
const HIPS: Vec3 = Vec3::new(HIP_WIDTH / 2.0, -HULL_ABOVE, -0.04);
/// Where the hands hang from the shoulders, in the hull's space.
const HANG: Vec3 = Vec3::new(0.03, -0.34, 0.12);
/// Seated, the hands rest in the lap.
const LAP: Vec3 = Vec3::new(0.12, -0.12, 0.34);
const ARM_SWING: f32 = 0.12;
/// The hand holding an open umbrella, in the hull's space: in front of the
/// chest, to the side of the head.
const HOLD: Vec3 = Vec3::new(0.2, 0.2, 0.3);
/// With an open umbrella, the head turns only this far towards it.
const UMBRELLA_GLANCE: f32 = 0.45;

/// Share of a step's cycle that a foot stands on the ground.
pub(super) const STANCE: f32 = 0.62;
pub(super) const STRIDE: f32 = 0.62;
const LIFT: f32 = 0.09;
const BOB: f32 = 0.025;
const SWAY: f32 = 0.025;
const LEAN: f32 = 0.12;
const HUNCH_LEAN: f32 = 0.35;
const HUNCH_DROP: f32 = 0.08;
/// The head thrusts forward and back with each step, like a pigeon's.
const NOD: f32 = 0.025;
/// Feet are heard landing above this much of the walking speed.
pub(super) const STEPPING: f32 = 0.2;
/// Breaths per second, and how far the hull rises with one.
const BREATH_RATE: f32 = 0.3;
const BREATH: f32 = 0.006;
/// The tail's spring: how stiff, how damped, and how much the hull's
/// moves throw it.
const TAIL_STIFFNESS: f32 = 60.0;
const TAIL_DAMPING: f32 = 7.0;
const TAIL_THROW: f32 = 20.0;
/// The tail rests a little raised.
const TAIL_REST: f32 = 0.15;
/// Wet feathers cling to the body, and fluff up while shaking.
const SLEEK: f32 = 0.05;
const FLUFF: f32 = 0.18;
/// Shakes this many times a second, rolling the hull this far.
const SHAKE_RATE: f32 = 9.0;
const SHAKE_ROLL: f32 = 0.22;

/// The parts of a [`Strider`], in its own space except for the parts on
/// the hull, which are in the hull's space.
pub(super) struct Pose {
    pub(super) hull: Transform,
    pub(super) plumage: Transform,
    pub(super) head: Transform,
    pub(super) tail: Transform,
    pub(super) thighs: [Transform; 2],
    pub(super) cuffs: [Transform; 2],
    pub(super) shins: [Transform; 2],
    pub(super) feet: [Transform; 2],
    pub(super) arms: [Transform; 2],
    pub(super) sleeves: [Transform; 2],
    /// The umbrella in its hand.
    pub(super) umbrella: Option<Transform>,
    pub(super) silhouette: Silhouette,
}

impl Strider {
    /// Moves on by `dt` seconds: its steps, its breath and its tail, with
    /// the strider at `body`.
    pub(super) fn step(&mut self, dt: f32, body: &Transform) {
        self.breath += dt * BREATH_RATE * TAU;
        self.phase += self.speed * dt * STANCE / (STRIDE * self.size);
        self.phase = self.phase.fract();
        let hull = *body * self.hull_pose(BREATH * self.size * self.breath.sin()).0;
        let at = hull.translation;
        let tail = &mut self.tail;
        if let Some(last) = tail.last
            && dt > 0.0
        {
            // The hull's motion in its own space throws the tail the other
            // way.
            let moved = (hull.rotation.inverse() * (at - last) / dt).clamp_length_max(3.0);
            let throw = Vec2::new(moved.y, moved.x) * TAIL_THROW;
            let target = Vec2::new(TAIL_REST, 0.0);
            let pull = (target - tail.angle) * TAIL_STIFFNESS - tail.speed * TAIL_DAMPING;
            tail.speed += (pull - throw) * dt;
            tail.angle += tail.speed * dt;
            tail.angle = tail.angle.clamp(Vec2::new(-0.6, -0.6), Vec2::new(0.9, 0.6));
        }
        tail.last = Some(at);
    }

    /// Where every part goes now.
    pub(super) fn posed(&self, gaze: &Gaze) -> Pose {
        let size = self.size;
        let breath = BREATH * size * self.breath.sin();
        let (hull, feet, knees, lean) = self.hull_pose(breath);
        let shake = self.shake();
        let hull = hull * Transform::from_rotation(Quat::from_rotation_z(shake * SHAKE_ROLL));
        let open = self.umbrella.as_ref().map_or(0.0, Umbrella::openness);
        let glance = gaze.turn.x.min(gaze::YAW.lerp(UMBRELLA_GLANCE, open));
        let nod = NOD * size * self.gait() * (2.0 * TAU * self.phase).sin();
        let fluff = 1.0 - SLEEK * self.wet + FLUFF * shake.abs().sqrt();
        let mut pose = Pose {
            hull,
            plumage: Transform::from_scale(self.hull * fluff),
            head: Transform::from_translation(HEAD * size + Vec3::Z * nod).with_rotation(
                Quat::from_rotation_y(glance - shake * 0.6)
                    * Quat::from_rotation_x(gaze.turn.y + self.hunch * 0.3 - lean * 0.8),
            ),
            tail: Transform::from_translation(RUMP * self.hull)
                .with_rotation(
                    Quat::from_rotation_y(self.tail.angle.y)
                        * Quat::from_rotation_x(-self.tail.angle.x - lean),
                )
                .with_scale(Vec3::splat(size * fluff)),
            thighs: [Transform::default(); 2],
            cuffs: [Transform::default(); 2],
            shins: [Transform::default(); 2],
            feet: [Transform::default(); 2],
            arms: [Transform::default(); 2],
            sleeves: [Transform::default(); 2],
            umbrella: None,
            silhouette: Silhouette::default(),
        };
        let skull = hull * pose.head * Transform::from_xyz(0.0, 0.0, 0.04 * size);
        let shadow = &mut pose.silhouette;
        shadow.blobs[0] = Blob::new(&hull, self.hull * fluff);
        shadow.blobs[1] = Blob::new(&skull, SKULL * size);
        shadow.height = skull.translation.y + SKULL.y * size / 2.0;
        shadow.bones[4] = Bone {
            from: hull.transform_point(NECK_BASE * size),
            to: hull.transform_point(HEAD * size),
            radius: NECK * size,
        };
        for (index, side) in [-1.0f32, 1.0].into_iter().enumerate() {
            let flip = Vec3::new(side, 1.0, 1.0);
            let hip = hull.transform_point(HIPS * flip * size);
            let ankle = feet[index] + Vec3::Y * ANKLE * size;
            let (knee, ankle) = (LEG * size).reach(hip, ankle, knees);
            pose.thighs[index] = rod(hip, knee, THIGH * size);
            pose.cuffs[index] = Self::cuff(hip, knee, size);
            pose.shins[index] = rod(knee, ankle, SHIN * size);
            pose.silhouette.bones[index] = Bone {
                from: hip,
                to: knee,
                radius: THIGH * size,
            };
            pose.silhouette.bones[2 + index] = Bone {
                from: knee,
                to: ankle,
                radius: SHIN * size / 2.0,
            };
            let lifted = (ankle.y - ANKLE * size).max(0.0);
            pose.feet[index] = Transform::from_translation(ankle - Vec3::Y * ANKLE * size)
                .with_rotation(Quat::from_rotation_x(-lifted * 3.0))
                .with_scale(Vec3::splat(size));

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
            pose.sleeves[index] = Self::cuff(shoulder, hand, size);
            pose.silhouette.bones[5 + index] = Bone {
                from: hull.transform_point(shoulder),
                to: hull.transform_point(hand),
                radius: ARM * size / 2.0,
            };
        }
        pose
    }

    /// Feathers at `from` hanging down along a limb towards `to`.
    fn cuff(from: Vec3, to: Vec3, size: f32) -> Transform {
        let down = (to - from).try_normalize().unwrap_or(Vec3::NEG_Y);
        Transform::from_translation(from)
            .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Y, down))
            .with_scale(Vec3::splat(size))
    }

    /// How far into its stride it is, from standing at 0 to walking at 1.
    pub(super) fn gait(&self) -> f32 {
        (self.speed / self.pace).clamp(0.0, 1.0)
    }

    /// The roll of a shake, fading out, from -1 to 1.
    fn shake(&self) -> f32 {
        if self.shaking <= 0.0 {
            return 0.0;
        }
        let t = 1.0 - self.shaking / SHAKE_TIME;
        (TAU * SHAKE_RATE * t * SHAKE_TIME).sin() * (PI * t).sin()
    }

    /// The hull, the spots of the feet, the bend of the knees and the lean
    /// of the hull.
    fn hull_pose(&self, breath: f32) -> (Transform, [Vec3; 2], Vec3, f32) {
        let size = self.size;
        match self.posture {
            Posture::Walking => {}
            Posture::Seated(height) => {
                let hull = Vec3::new(0.0, height + HULL_ABOVE * size + breath, 0.0);
                let feet = [-1.0, 1.0].map(|side| Vec3::new(side * 0.17, 0.0, 0.28) * size);
                return (Transform::from_translation(hull), feet, SEATED_KNEES, 0.0);
            }
        }
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
