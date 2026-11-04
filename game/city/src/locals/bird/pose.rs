use std::f32::consts::{PI, TAU};

use bevy::prelude::*;

use super::super::gaze::{self, Gaze};
use super::super::shadows::{Blob, Bone as Capsule, Silhouette};
use super::super::umbrella::{self, Umbrella};
use super::skeleton::{
    ARM, Bone, ELBOWS, HEAD, HEAD_RADII, HIP, HULL, HULL_RADII, KNEES, LEG, RUMP, SHOULDER, Side,
    frame,
};
use super::{Bird, Posture, SHAKE_TIME, UMBRELLA_SIZE};

/// Share of a step's cycle that a foot stands on the ground.
const STANCE: f32 = 0.6;
const STRIDE: f32 = 0.36;
const LIFT: f32 = 0.06;
const BOB: f32 = 0.018;
const SWAY: f32 = 0.02;
/// Small birds waddle, rolling onto the foot that stands.
const WADDLE: f32 = 0.08;
const LEAN: f32 = 0.1;
const HUNCH_LEAN: f32 = 0.3;
const HUNCH_DROP: f32 = 0.05;
/// The head thrusts forward and back with each step, like a pigeon's.
const NOD: f32 = 0.02;
/// Feet are heard landing above this much of the walking speed.
pub(super) const STEPPING: f32 = 0.2;
const BREATH_RATE: f32 = 0.3;
const BREATH: f32 = 0.012;
/// Hands hang beside the hull while walking, and rest in the lap.
const HANG: Vec3 = Vec3::new(0.03, -0.22, 0.06);
const LAP: Vec3 = Vec3::new(-0.06, -0.16, 0.15);
const ARM_SWING: f32 = 0.08;
/// The hand holding an open umbrella: in front of the chest, to the side.
const HOLD: Vec3 = Vec3::new(0.15, 0.96, 0.26);
/// With an open umbrella, the head turns only this far towards it.
const UMBRELLA_GLANCE: f32 = 0.5;
/// Seated, the feet dangle this far below and ahead of the hips, and swing.
const DANGLE: Vec3 = Vec3::new(0.0, -0.5, 0.18);
const DANGLE_SWING: f32 = 0.05;
/// The tail's spring: how stiff, how damped, and how much the hull's
/// moves throw it.
const TAIL_STIFFNESS: f32 = 60.0;
const TAIL_DAMPING: f32 = 7.0;
const TAIL_THROW: f32 = 20.0;
/// The tail rests a little raised.
const TAIL_REST: f32 = 0.2;
/// Wet feathers cling to the body, and fluff up while shaking.
const SLEEK: f32 = 0.05;
const FLUFF: f32 = 0.15;
/// Shakes this many times a second, rolling the hull this far.
const SHAKE_RATE: f32 = 9.0;
const SHAKE_ROLL: f32 = 0.25;
/// Keepers rock onto each foot as they shuffle along the counter.
const SHUFFLE_ROCK: f32 = 0.08;
const SHUFFLE_LIFT: f32 = 0.04;

/// What a keeper's hands and feet do, in the bird's own space.
#[derive(Debug, Clone, Copy)]
pub(in super::super) struct Work {
    pub(in super::super) hands: [Vec3; 2],
    /// From standing at 0 to shuffling at 1.
    pub(in super::super) shuffle: f32,
    /// Steps taken.
    pub(in super::super) steps: f32,
}

/// The tail swings after the hull on a spring.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Tail {
    /// Pitch up and swing to the side, in radians, and how fast they change.
    angle: Vec2,
    speed: Vec2,
    /// Where the hull was last frame, to feel it move.
    last: Option<Vec3>,
}

/// Every joint of a [`Bird`] in its body's space, with what goes with them.
pub(super) struct Pose {
    pub(super) joints: [Transform; Bone::COUNT],
    /// Where the soles are.
    pub(super) feet: [Vec3; 2],
    /// The umbrella in its hand.
    pub(super) umbrella: Option<Transform>,
    pub(super) silhouette: Silhouette,
}

impl Bird {
    /// Moves on by `dt` seconds: its steps, its breath and its tail, with
    /// its body at `body`.
    pub(super) fn step(&mut self, dt: f32, body: &Transform, work: Option<&Work>) {
        self.breath += dt * BREATH_RATE * TAU;
        self.phase += self.speed * dt * STANCE / (STRIDE * self.size);
        self.phase = self.phase.fract();
        let hull = *body * self.hull(work).0;
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

    /// From standing at 0 to walking at 1.
    pub(super) fn gait(&self) -> f32 {
        (self.speed / self.pace).clamp(0.0, 1.0)
    }

    /// Where every joint goes now.
    pub(super) fn posed(&self, gaze: &Gaze, work: Option<&Work>) -> Pose {
        let (hull, feet, knees, lean) = self.hull(work);
        let shake = self.shake();
        let hull = hull * Transform::from_rotation(Quat::from_rotation_z(shake * SHAKE_ROLL));
        let fluff = 1.0 - SLEEK * self.wet + FLUFF * shake.abs().sqrt();
        let breath = 1.0 + BREATH * self.breath.sin();
        let mut joints = Bone::rests();
        let mut set = |bone: Bone, transform: Transform| joints[bone.joint() as usize] = transform;
        set(
            Bone::Hull,
            hull.with_scale(self.girth * fluff * Vec3::new(breath, 1.0, breath)),
        );

        // The head stays level as the hull rolls, like a bird's.
        let open = self.umbrella.as_ref().map_or(0.0, Umbrella::openness);
        let glance = gaze.turn.x.min(gaze::YAW.lerp(UMBRELLA_GLANCE, open));
        let nod = NOD * self.gait() * (2.0 * TAU * self.phase).sin();
        let head_at = hull.transform_point(HEAD - HULL) + Vec3::Z * nod;
        let head = Transform::from_translation(head_at).with_rotation(
            Quat::from_rotation_y(glance - shake * 0.6)
                * Quat::from_rotation_x(gaze.turn.y + self.hunch * 0.3),
        );
        set(Bone::Head, head);
        set(
            Bone::Tail,
            hull * Transform::from_translation(RUMP - HULL).with_rotation(
                Quat::from_rotation_y(self.tail.angle.y)
                    * Quat::from_rotation_x(-self.tail.angle.x - lean),
            ),
        );

        let ankles_y = Bone::Foot(Side::Right).rest().translation.y;
        let mut silhouette = Silhouette {
            feet: Vec3::ZERO,
            height: head_at.y + HEAD_RADII.y,
            ..default()
        };
        silhouette.blobs[0] = Blob::new(&hull, HULL_RADII * 2.0 * self.girth * fluff);
        silhouette.blobs[1] = Blob::new(&head, HEAD_RADII * 2.0);
        silhouette.bones[6] = Capsule {
            from: hull.translation,
            to: head_at,
            radius: 0.09,
        };
        let mut umbrella_at = None;
        let mut soles = [Vec3::ZERO; 2];
        for (index, side) in Side::BOTH.into_iter().enumerate() {
            let hip = hull.transform_point(side.mirror(HIP) - HULL);
            let (knee, ankle) = LEG.reach(hip, feet[index], knees);
            set(Bone::Thigh(side), frame(hip, knee - hip, knees));
            set(Bone::Shin(side), frame(knee, ankle - knee, knees));
            let lifted = (feet[index].y - ankles_y).clamp(0.0, 0.25);
            set(
                Bone::Foot(side),
                Transform::from_translation(ankle)
                    .with_rotation(Quat::from_rotation_x(-lifted * 4.0)),
            );
            soles[index] = ankle.with_y(0.0);
            silhouette.bones[index] = Capsule {
                from: hip,
                to: knee,
                radius: 0.04,
            };
            silhouette.bones[2 + index] = Capsule {
                from: knee,
                to: ankle,
                radius: 0.022,
            };

            let shoulder = hull.transform_point(side.mirror(SHOULDER) - HULL);
            let swing = ARM_SWING * self.gait() * ((self.phase + index as f32 * 0.5) * TAU).sin();
            let mut hand = match (self.posture, work) {
                (Posture::Keeping, Some(work)) => work.hands[index],
                (Posture::Seated(_), _) => shoulder + side.mirror(LAP),
                _ => shoulder + side.mirror(HANG) + Vec3::Z * swing,
            };
            let umbrella = self.umbrella.as_ref().filter(|_| side == Side::Right);
            if let Some(umbrella) = umbrella {
                hand = umbrella.hand(hand, HOLD);
            }
            let elbows = side.mirror(ELBOWS);
            let (elbow, wrist) = ARM.reach(shoulder, hand, elbows);
            set(
                Bone::UpperArm(side),
                frame(shoulder, elbow - shoulder, elbows),
            );
            set(Bone::Forearm(side), frame(elbow, wrist - elbow, elbows));
            let palm = frame(wrist, wrist - elbow, elbows);
            set(Bone::Hand(side), palm);
            silhouette.bones[4 + index] = Capsule {
                from: shoulder,
                to: wrist,
                radius: 0.035,
            };
            if let Some(umbrella) = umbrella {
                let grip = palm.transform_point(Vec3::Y * 0.03);
                let placed = umbrella.place(grip, swing * 3.0);
                if let Some(canopy) = umbrella.shadow(&placed, UMBRELLA_SIZE) {
                    silhouette.blobs[2] = canopy;
                    silhouette.height = placed.translation.y + umbrella::SHAFT * UMBRELLA_SIZE;
                }
                umbrella_at = Some(placed);
            }
        }
        Pose {
            joints,
            feet: soles,
            umbrella: umbrella_at,
            silhouette,
        }
    }

    /// The roll of a shake, fading out, from -1 to 1.
    fn shake(&self) -> f32 {
        if self.shaking <= 0.0 {
            return 0.0;
        }
        let t = 1.0 - self.shaking / SHAKE_TIME;
        (TAU * SHAKE_RATE * t * SHAKE_TIME).sin() * (PI * t).sin()
    }

    /// The hull without its scale, where the ankles go, which way the knees
    /// bend, and how far the hull leans forward.
    fn hull(&self, work: Option<&Work>) -> (Transform, [Vec3; 2], Vec3, f32) {
        let ankles = Side::BOTH.map(|side| side.mirror(Bone::Foot(Side::Right).rest().translation));
        match (self.posture, work) {
            (Posture::Seated(height), _) => {
                let hull =
                    Transform::from_translation(Vec3::new(0.0, height + HULL.y - HIP.y, 0.0));
                let feet = Side::BOTH.map(|side| {
                    let hip = hull.transform_point(side.mirror(HIP) - HULL);
                    let swing = (self.breath * 0.7 + side.sign()).sin() * DANGLE_SWING;
                    hip + DANGLE + Vec3::Z * swing
                });
                (hull, feet, Vec3::new(0.0, 0.6, 1.0), 0.0)
            }
            (Posture::Keeping, Some(work)) => {
                let rock = (PI * work.steps).sin();
                let hull = Transform::from_translation(HULL + Vec3::Y * 0.01 * rock.abs())
                    .with_rotation(Quat::from_rotation_z(SHUFFLE_ROCK * work.shuffle * rock));
                let feet = [0, 1].map(|index| {
                    let up = if (index == 0) == (rock > 0.0) {
                        0.0
                    } else {
                        rock.abs()
                    };
                    ankles[index] + Vec3::Y * SHUFFLE_LIFT * work.shuffle * up
                });
                (hull, feet, KNEES, 0.0)
            }
            _ => self.walking(ankles),
        }
    }

    fn walking(&self, ankles: [Vec3; 2]) -> (Transform, [Vec3; 2], Vec3, f32) {
        let gait = self.gait();
        let bob = BOB * gait * (2.0 * TAU * self.phase).cos();
        let sway = SWAY * gait * (TAU * self.phase).sin();
        let lean = LEAN * gait + HUNCH_LEAN * self.hunch;
        let hull =
            Transform::from_translation(HULL + Vec3::new(sway, bob - HUNCH_DROP * self.hunch, 0.0))
                .with_rotation(
                    Quat::from_rotation_x(lean)
                        * Quat::from_rotation_z(-WADDLE * gait * (TAU * self.phase).sin()),
                );
        let feet = [0.0, 0.5].map(|offset| {
            let cycle = (self.phase + offset).fract();
            if cycle < STANCE {
                Vec3::Z * STRIDE * (0.5 - cycle / STANCE)
            } else {
                let t = (cycle - STANCE) / (1.0 - STANCE);
                let ahead = t * t * (3.0 - 2.0 * t);
                Vec3::new(0.0, (PI * t).sin() * LIFT * gait, STRIDE * (ahead - 0.5))
            }
        });
        let feet = [ankles[0] + feet[0], ankles[1] + feet[1]];
        (hull, feet, KNEES, lean)
    }
}
