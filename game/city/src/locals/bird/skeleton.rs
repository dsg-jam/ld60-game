use bevy::prelude::*;

use super::super::limb::Limb;

/// Where the hull is, its size and how it rests on the hips. All bird
/// measures are for a bird of size 1, with its feet at the origin and its
/// front towards +Z.
pub(super) const HULL: Vec3 = Vec3::new(0.0, 0.8, 0.0);
pub(super) const HULL_RADII: Vec3 = Vec3::new(0.23, 0.22, 0.27);
pub(super) const HEAD: Vec3 = Vec3::new(0.0, 1.12, 0.15);
pub(super) const HEAD_RADII: Vec3 = Vec3::new(0.155, 0.145, 0.155);
/// Where the tail leaves the hull.
pub(super) const RUMP: Vec3 = Vec3::new(0.0, 0.85, -0.24);
/// The right hip and shoulder. The left ones mirror them.
pub(super) const HIP: Vec3 = Vec3::new(0.09, 0.66, -0.02);
pub(super) const SHOULDER: Vec3 = Vec3::new(0.17, 0.88, 0.1);
pub(super) const LEG: Limb = Limb {
    upper: 0.3,
    lower: 0.34,
};
pub(super) const ARM: Limb = Limb {
    upper: 0.15,
    lower: 0.14,
};
/// The knees bend back, like a bird's, and the elbows out and back.
pub(super) const KNEES: Vec3 = Vec3::NEG_Z;
pub(super) const ELBOWS: Vec3 = Vec3::new(0.6, -0.2, -0.8);
/// The rest pose of the right leg and arm: knee, ankle, elbow and wrist.
const KNEE: Vec3 = Vec3::new(0.09, 0.37, -0.1);
const ANKLE: Vec3 = Vec3::new(0.1, 0.04, 0.0);
const ELBOW: Vec3 = Vec3::new(0.22, 0.75, 0.1);
const WRIST: Vec3 = Vec3::new(0.23, 0.62, 0.16);

/// The joints of a bird. Each moves the parts of the mesh bound to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Bone {
    Hull,
    Head,
    Tail,
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
    pub(super) const COUNT: usize = 15;

    pub(super) fn all() -> impl Iterator<Item = Self> {
        [Self::Hull, Self::Head, Self::Tail]
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
        let limb = |side: Side, offset: usize| (3 + side.index() * 6 + offset) as u16;
        match self {
            Self::Hull => 0,
            Self::Head => 1,
            Self::Tail => 2,
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
        match self {
            Self::Hull => Transform::from_translation(HULL),
            Self::Head => Transform::from_translation(HEAD),
            Self::Tail => Transform::from_translation(RUMP),
            Self::Thigh(side) => {
                let hip = side.mirror(HIP);
                let knee = side.mirror(KNEE);
                frame(hip, knee - hip, KNEES)
            }
            Self::Shin(side) => {
                let knee = side.mirror(KNEE);
                frame(knee, side.mirror(ANKLE) - knee, KNEES)
            }
            Self::Foot(side) => Transform::from_translation(side.mirror(ANKLE)),
            Self::UpperArm(side) => {
                let shoulder = side.mirror(SHOULDER);
                let elbow = side.mirror(ELBOW);
                frame(shoulder, elbow - shoulder, side.mirror(ELBOWS))
            }
            Self::Forearm(side) => {
                let elbow = side.mirror(ELBOW);
                frame(elbow, side.mirror(WRIST) - elbow, side.mirror(ELBOWS))
            }
            Self::Hand(side) => {
                let elbow = side.mirror(ELBOW);
                let wrist = side.mirror(WRIST);
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
