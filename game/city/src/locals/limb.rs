use std::ops::Mul;

use bevy::prelude::*;

/// A limb of two segments, like a leg with its knee.
#[derive(Clone, Copy, Debug)]
pub(super) struct Limb {
    pub(super) upper: f32,
    pub(super) lower: f32,
}

impl Limb {
    /// The joint between the segments and the end of the limb, reaching
    /// from `root` for `target` and bending towards `bend`. A target out of
    /// reach pulls the limb straight towards it.
    pub(super) fn reach(&self, root: Vec3, target: Vec3, bend: Vec3) -> (Vec3, Vec3) {
        let to = target - root;
        let along = to.try_normalize().unwrap_or(Vec3::NEG_Y);
        let longest = (self.upper + self.lower) * 0.999;
        let shortest = (self.upper - self.lower).abs() + 1e-4;
        let distance = to.length().clamp(shortest, longest);
        let end = root + along * distance;
        let out = (distance * distance + self.upper * self.upper - self.lower * self.lower)
            / (2.0 * distance);
        let height = (self.upper * self.upper - out * out).max(0.0).sqrt();
        let side = (bend - along * bend.dot(along))
            .try_normalize()
            .unwrap_or_else(|| along.any_orthonormal_vector());
        (root + along * out + side * height, end)
    }
}

impl Mul<f32> for Limb {
    type Output = Limb;

    fn mul(self, scale: f32) -> Limb {
        Limb {
            upper: self.upper * scale,
            lower: self.lower * scale,
        }
    }
}

/// A rod of `thickness` from `from` to `to`, for a unit cylinder along Y.
pub(super) fn rod(from: Vec3, to: Vec3, thickness: f32) -> Transform {
    let span = to - from;
    let rotation = span.try_normalize().map_or(Quat::IDENTITY, |along| {
        Quat::from_rotation_arc(Vec3::Y, along)
    });
    Transform {
        translation: (from + to) / 2.0,
        rotation,
        scale: Vec3::new(thickness, span.length().max(1e-4), thickness),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEG: Limb = Limb {
        upper: 0.6,
        lower: 0.7,
    };

    #[test]
    fn segments_keep_their_length() {
        let hip = Vec3::new(0.1, 1.1, 0.0);
        let foot = Vec3::new(0.15, 0.0, 0.3);
        let (knee, end) = LEG.reach(hip, foot, Vec3::NEG_Z);
        assert!((knee.distance(hip) - LEG.upper).abs() < 1e-4);
        assert!((knee.distance(end) - LEG.lower).abs() < 1e-4);
        assert!(end.distance(foot) < 1e-4);
    }

    #[test]
    fn knee_bends_the_asked_way() {
        let (knee, _) = LEG.reach(Vec3::Y, Vec3::ZERO, Vec3::NEG_Z);
        assert!(knee.z < -0.1);
    }

    #[test]
    fn target_out_of_reach_straightens() {
        let (knee, end) = LEG.reach(Vec3::Y * 2.0, Vec3::ZERO, Vec3::Z);
        assert!(end.y > 0.69 && end.y < 0.71);
        assert!(knee.z.abs() < 0.05);
    }
}
