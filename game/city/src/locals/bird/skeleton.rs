use bevy::prelude::*;

use super::super::limb::Limb;
use super::dress::KNOT;

/// Where the hull is, its size and how far it leans forward. All bird
/// measures are for a bird of size 1, with its feet at the origin and its
/// front towards +Z.
pub(super) const HULL: Vec3 = Vec3::new(0.0, 0.68, -0.02);
pub(super) const HULL_RADII: Vec3 = Vec3::new(0.095, 0.24, 0.11);
/// The hull is a slim egg, its top leaning forward this far.
pub(super) const HULL_TILT: f32 = 0.6;
/// Where the neck leaves the front of the hull, and the head.
pub(super) const NECK: Vec3 = Vec3::new(0.0, 0.84, 0.1);
pub(super) const HEAD: Vec3 = Vec3::new(0.0, 1.215, 0.1);
pub(super) const HEAD_RADII: Vec3 = Vec3::new(0.03, 0.036, 0.05);
/// Where the tail leaves the hull.
pub(super) const RUMP: Vec3 = Vec3::new(0.0, 0.53, -0.14);
/// The right hip and shoulder. The left ones mirror them.
pub(super) const HIP: Vec3 = Vec3::new(0.055, 0.6, -0.03);
pub(super) const SHOULDER: Vec3 = Vec3::new(0.085, 0.82, 0.04);
/// The upper leg is the feathered drumstick, the lower one the bare shank
/// below the joint that bends back.
pub(super) const LEG: Limb = Limb {
    upper: 0.36,
    lower: 0.27,
};
pub(super) const ARM: Limb = Limb {
    upper: 0.16,
    lower: 0.15,
};
/// The knees bend back, like a bird's, and the elbows out and back.
pub(super) const KNEES: Vec3 = Vec3::NEG_Z;
pub(super) const ELBOWS: Vec3 = Vec3::new(0.3, -0.2, -0.9);
/// The rest pose of the right ankle and wrist.
const ANKLE: Vec3 = Vec3::new(0.06, 0.035, 0.0);
const WRIST: Vec3 = Vec3::new(0.11, 0.55, 0.0);

/// The joints of a bird. Each moves the parts of the mesh bound to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Bone {
    Hull,
    Neck,
    Head,
    Tail,
    /// The knot of a scarf, its ends swinging from it.
    Scarf,
    Thigh(Side),
    Shin(Side),
    Foot(Side),
    UpperArm(Side),
    Forearm(Side),
    Hand(Side),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Left,
    Right,
}

impl Side {
    pub(super) const BOTH: [Self; 2] = [Self::Left, Self::Right];

    /// -1 on the left, +1 on the right, along X.
    pub(super) fn sign(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }

    /// `point` of the right side, on this side.
    pub(super) fn mirror(self, point: Vec3) -> Vec3 {
        point * Vec3::new(self.sign(), 1.0, 1.0)
    }

    fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

impl Bone {
    pub(super) const COUNT: usize = 17;

    pub(super) fn all() -> impl Iterator<Item = Self> {
        [Self::Hull, Self::Neck, Self::Head, Self::Tail, Self::Scarf]
            .into_iter()
            .chain(Side::BOTH.into_iter().flat_map(|side| {
                [
                    Self::Thigh(side),
                    Self::Shin(side),
                    Self::Foot(side),
                    Self::UpperArm(side),
                    Self::Forearm(side),
                    Self::Hand(side),
                ]
            }))
    }

    /// Its index among the joints of the skinned meshes.
    pub(super) fn joint(self) -> u16 {
        let limb = |side: Side, offset: usize| (5 + side.index() * 6 + offset) as u16;
        match self {
            Self::Hull => 0,
            Self::Neck => 1,
            Self::Head => 2,
            Self::Tail => 3,
            Self::Scarf => 4,
            Self::Thigh(side) => limb(side, 0),
            Self::Shin(side) => limb(side, 1),
            Self::Foot(side) => limb(side, 2),
            Self::UpperArm(side) => limb(side, 3),
            Self::Forearm(side) => limb(side, 4),
            Self::Hand(side) => limb(side, 5),
        }
    }

    /// Where it is and how it is turned in the rest pose, which the meshes
    /// are made in. Limb bones run along their +Y, with +Z towards the bend.
    pub(super) fn rest(self) -> Transform {
        let leg = |side: Side| {
            let hip = side.mirror(HIP);
            let (knee, ankle) = LEG.reach(hip, side.mirror(ANKLE), KNEES);
            (hip, knee, ankle)
        };
        let arm = |side: Side| {
            let shoulder = side.mirror(SHOULDER);
            let (elbow, wrist) = ARM.reach(shoulder, side.mirror(WRIST), side.mirror(ELBOWS));
            (shoulder, elbow, wrist)
        };
        match self {
            Self::Hull => Transform::from_translation(HULL),
            Self::Neck => Transform::from_translation(NECK),
            Self::Head => Transform::from_translation(HEAD),
            Self::Tail => Transform::from_translation(RUMP),
            Self::Scarf => Transform::from_translation(KNOT),
            Self::Thigh(side) => {
                let (hip, knee, _) = leg(side);
                frame(hip, knee - hip, KNEES)
            }
            Self::Shin(side) => {
                let (_, knee, ankle) = leg(side);
                frame(knee, ankle - knee, KNEES)
            }
            Self::Foot(side) => Transform::from_translation(leg(side).2),
            Self::UpperArm(side) => {
                let (shoulder, elbow, _) = arm(side);
                frame(shoulder, elbow - shoulder, side.mirror(ELBOWS))
            }
            Self::Forearm(side) => {
                let (_, elbow, wrist) = arm(side);
                frame(elbow, wrist - elbow, side.mirror(ELBOWS))
            }
            Self::Hand(side) => {
                let (_, elbow, wrist) = arm(side);
                frame(wrist, wrist - elbow, side.mirror(ELBOWS))
            }
        }
    }

    /// The rest pose of each joint, in the order of [`Bone::joint`].
    pub(super) fn rests() -> [Transform; Self::COUNT] {
        let mut rests = [Transform::default(); Self::COUNT];
        for bone in Self::all() {
            rests[bone.joint() as usize] = bone.rest();
        }
        rests
    }
}

/// A frame at `at` whose +Y runs `along` and whose +Z leans towards `bend`.
pub(super) fn frame(at: Vec3, along: Vec3, bend: Vec3) -> Transform {
    let y = along.normalize_or(Vec3::NEG_Y);
    let z = (bend - y * bend.dot(y)).normalize_or(y.any_orthonormal_vector());
    let x = y.cross(z);
    Transform::from_translation(at).with_rotation(Quat::from_mat3(&Mat3::from_cols(x, y, z)))
}
