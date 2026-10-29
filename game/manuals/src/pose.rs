//! The file standing closed on the counter, lying down on the mat and
//! opening there, and back.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use super::cloth;
use super::model::{BOARD, CLOSED, COVER, Parts, SPINE};

/// Seconds to lie down or to stand up.
const LIE_TIME: f32 = 0.5;
/// Seconds after the file starts to lie down that the cover starts to open.
/// The cover closes as late before the file stands up.
const COVER_START: f32 = 0.4;
/// Seconds the cover takes to open or close.
const COVER_TIME: f32 = 0.4;
/// Below this fold in radians the cloth lies flat where the covers meet the
/// spine, and there is no bend to show.
const FLAT: f32 = 1e-3;
/// How high the file lifts on its way between standing and lying.
const LIFT: f32 = 0.06;

/// How far the file is from standing closed to lying open, in seconds, and
/// how far its covers lean in when shut, see
/// [`Stack::lean`](super::stack::Stack::lean).
#[derive(Component, Debug, Default)]
pub struct Pose {
    time: f32,
    lean: f32,
}

impl Pose {
    const END: f32 = COVER_START + COVER_TIME;

    /// Moves `delta` seconds towards lying open or standing closed. Returns
    /// whether it moved.
    pub(super) fn step(&mut self, open: bool, delta: f32) -> bool {
        let next = if open {
            (self.time + delta).min(Self::END)
        } else {
            (self.time - delta).max(0.0)
        };
        let moved = next != self.time;
        self.time = next;
        moved
    }

    /// Leans the shut covers in by `lean` radians. Returns whether that
    /// changed it.
    pub(super) fn lean_to(&mut self, lean: f32) -> bool {
        let changed = lean != self.lean;
        self.lean = lean;
        changed
    }

    pub(super) fn is_open(&self) -> bool {
        self.time >= Self::END
    }

    /// From 0 standing to 1 lying.
    pub fn lying(&self) -> f32 {
        smoothstep(self.time / LIE_TIME)
    }

    /// From 0 shut to 1 open.
    pub(super) fn cover(&self) -> f32 {
        smoothstep((self.time - COVER_START) / COVER_TIME)
    }

    /// How far each cover is folded about the spine, in radians.
    fn fold(&self) -> f32 {
        (1.0 - self.cover()) * (FRAC_PI_2 + self.lean)
    }
}

impl Parts {
    /// Places the covers for `pose`, with the file lying open at `lying`,
    /// and bends the cloth between them and the spine.
    /// [`Stack::arrange`](super::stack::Stack::arrange) places what is on
    /// the rings.
    pub(super) fn pose(
        &self,
        pose: &Pose,
        lying: Transform,
        transforms: &mut Query<&mut Transform>,
        (visibilities, meshes): (&mut Query<&mut Visibility>, &mut Assets<Mesh>),
    ) {
        let mut set = |entity: Entity, transform: Transform| {
            if let Ok(mut current) = transforms.get_mut(entity) {
                *current = transform;
            }
        };
        let down = pose.lying();
        let stand = standing(pose.lean);
        let center = closed_center(pose.lean);
        let rotation = stand.rotation.slerp(lying.rotation, down);
        let at = stand
            .transform_point(center)
            .lerp(lying.transform_point(center), down)
            + Vec3::Y * LIFT * (down * PI).sin();
        set(
            self.open,
            Transform::from_translation(at - rotation * center).with_rotation(rotation),
        );

        let fold = pose.fold();
        let turn = Quat::from_rotation_z(-fold);
        set(self.spine, Self::spine_hinge().with_rotation(turn));
        set(self.front, Self::front_hinge().with_rotation(turn));
        let down = -FRAC_PI_2;
        let bent = fold > FLAT;
        for ((bend, mesh), (from, to)) in self
            .bends
            .iter()
            .zip([(down, down + fold), (down - fold, down)])
        {
            if let Ok(mut visibility) = visibilities.get_mut(*bend) {
                visibility.set_if_neq(if bent {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
            if bent && let Some(mut mesh) = meshes.get_mut(mesh) {
                *mesh = cloth::hinge(from, to, BOARD, COVER.y);
            }
        }
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The spine hinge of the closed file with its covers leaning in by
/// `lean`, in the space of the open file.
fn closed_spine(lean: f32) -> Transform {
    Parts::spine_hinge().with_rotation(Quat::from_rotation_z(-FRAC_PI_2 - lean))
}

/// The middle of the closed file lying down with its covers leaning in by
/// `lean`, in the space of the open file: halfway from the outside of the
/// spine to the free edges of the covers.
fn closed_center(lean: f32) -> Vec3 {
    closed_spine(lean).transform_point(Vec3::new(-SPINE / 2.0, CLOSED.z / 2.0 - BOARD, 0.0))
}

/// Where the closed file with its covers leaning in by `lean` stands, in
/// the space of the file: on the bottom edge of its pages with the spine
/// towards +Z. The spine stands in the same place however far the covers
/// lean.
pub(super) fn standing(lean: f32) -> Transform {
    let upright = Quat::from_mat3(&Mat3::from_cols(Vec3::NEG_Z, Vec3::X, Vec3::NEG_Y));
    let rotation = upright * Quat::from_rotation_z(lean);
    Transform::from_rotation(rotation)
        .with_translation(Vec3::Y * CLOSED.y / 2.0 - rotation * closed_center(lean))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_closed_file_stands_on_the_counter() {
        let stand = standing(0.0);
        let center = stand.transform_point(closed_center(0.0));
        assert!(center.abs_diff_eq(Vec3::Y * CLOSED.y / 2.0, 1e-6));
        let spine_out = stand.rotation * Vec3::NEG_X;
        assert!(spine_out.abs_diff_eq(Vec3::Z, 1e-6));
    }

    #[test]
    fn the_spine_stays_put_as_the_covers_lean() {
        let spine = |lean: f32| standing(lean) * closed_spine(lean);
        for lean in [-0.1, 0.05, 0.2] {
            let (a, b) = (spine(0.0), spine(lean));
            assert!(a.translation.abs_diff_eq(b.translation, 1e-6));
            assert!(a.rotation.abs_diff_eq(b.rotation, 1e-6));
        }
    }
}
