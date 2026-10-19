//! The file standing closed on the counter, lying down on the mat and
//! opening there, and back.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use super::model::{BOARD, CLOSED, COVER, Parts, RING_HEIGHT, RING_RADIUS, SPINE};

/// Seconds to lie down or to stand up.
const LIE_TIME: f32 = 0.5;
/// Seconds after the file starts to lie down that the cover starts to open.
/// The cover closes as late before the file stands up.
const COVER_START: f32 = 0.4;
/// Seconds the cover takes to open or close.
const COVER_TIME: f32 = 0.4;
/// How high the file lifts on its way between standing and lying.
const LIFT: f32 = 0.06;

/// How far the file is from standing closed to lying open, in seconds.
#[derive(Component, Debug, Default)]
pub(super) struct Pose(f32);

impl Pose {
    const END: f32 = COVER_START + COVER_TIME;

    /// Moves `delta` seconds towards lying open or standing closed. Returns
    /// whether it moved.
    pub(super) fn step(&mut self, open: bool, delta: f32) -> bool {
        let next = if open {
            (self.0 + delta).min(Self::END)
        } else {
            (self.0 - delta).max(0.0)
        };
        let moved = next != self.0;
        self.0 = next;
        moved
    }

    pub(super) fn is_open(&self) -> bool {
        self.0 >= Self::END
    }

    /// From 0 standing to 1 lying.
    fn lying(&self) -> f32 {
        smoothstep(self.0 / LIE_TIME)
    }

    /// From 0 shut to 1 open.
    fn cover(&self) -> f32 {
        smoothstep((self.0 - COVER_START) / COVER_TIME)
    }
}

impl Parts {
    /// Places the parts for `pose`, with the file lying open at `lying`.
    /// The turned pages turn back over the rings about `pivot` as the cover
    /// closes.
    pub(super) fn pose(
        &self,
        pose: &Pose,
        lying: Transform,
        pivot: Vec3,
        transforms: &mut Query<&mut Transform>,
    ) {
        let mut set = |entity: Entity, transform: Transform| {
            if let Ok(mut current) = transforms.get_mut(entity) {
                *current = transform;
            }
        };
        let down = pose.lying();
        let stand = standing();
        let center = closed_center();
        let rotation = stand.rotation.slerp(lying.rotation, down);
        let at = stand
            .transform_point(center)
            .lerp(lying.transform_point(center), down)
            + Vec3::Y * LIFT * (down * PI).sin();
        set(
            self.open,
            Transform::from_translation(at - rotation * center).with_rotation(rotation),
        );

        let open = pose.cover();
        let fold = Quat::from_rotation_z(-(1.0 - open) * FRAC_PI_2);
        set(self.spine, Self::spine_hinge().with_rotation(fold));
        set(self.front, Self::front_hinge().with_rotation(fold));
        let slide = slide(open);
        set(self.right, Transform::from_xyz(slide, 0.0, 0.0));
        set(
            self.turned,
            Transform::from_translation(pivot + Vec3::X * slide / 2.0)
                * Transform::from_rotation(Quat::from_rotation_z(-(1.0 - open) * PI))
                * Transform::from_translation(-pivot),
        );
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The middle of the closed file lying down, in the space of the open file.
fn closed_center() -> Vec3 {
    Vec3::new(
        (SPINE - BOARD + COVER.x) / 2.0,
        (SPINE + 2.0 * BOARD) / 2.0,
        0.0,
    )
}

/// Where the closed file stands, in the space of the file: on its bottom
/// edge with the spine towards +Z.
pub(super) fn standing() -> Transform {
    let rotation = Quat::from_mat3(&Mat3::from_cols(Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y));
    Transform::from_rotation(rotation)
        .with_translation(Vec3::Y * CLOSED.y / 2.0 - rotation * closed_center())
}

/// How far the pages not turned slide towards the back cover while the
/// cover closes, with the rings on the spine. `open` goes from 0 shut to 1.
fn slide(open: f32) -> f32 {
    let angle = -(1.0 - open) * FRAC_PI_2;
    SPINE / 2.0 + angle.cos() * (RING_RADIUS - SPINE / 2.0)
        - angle.sin() * RING_HEIGHT
        - RING_RADIUS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_closed_file_stands_on_the_counter() {
        let stand = standing();
        let center = stand.transform_point(closed_center());
        assert!(center.abs_diff_eq(Vec3::Y * CLOSED.y / 2.0, 1e-6));
        let spine_out = stand.rotation * Vec3::NEG_X;
        assert!(spine_out.abs_diff_eq(Vec3::Z, 1e-6));
    }

    #[test]
    fn open_pages_do_not_slide() {
        assert!(slide(1.0).abs() < 1e-6);
        assert!(slide(0.0) > 0.0);
    }
}
