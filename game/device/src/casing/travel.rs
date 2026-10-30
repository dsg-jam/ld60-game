use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use ld60_game_audio::{Play, Sound};
use ld60_geometry::block::Block;

use super::{Aside, Casing, Cover, SCREW};
use crate::Holdable;
use crate::thread::Thread;

/// Seconds a screw takes from its hole to the mat.
const SCREW_TIME: f32 = 1.4;
/// Shares of [`SCREW_TIME`]. The screwdriver sits on the screw in its hole
/// until the first, turns it out until the second and holds it until the
/// third. Then the screw is carried to the mat. The turning takes as long
/// as the sound of it.
const TURN_FROM: f32 = 0.18;
const TURN_TO: f32 = 0.48;
const LET_GO: f32 = 0.6;
const TURNS: f32 = 3.0;
const COVER_TIME: f32 = 0.8;
/// Share of [`COVER_TIME`] that lifts the cover straight off.
const LIFT_TO: f32 = 0.3;
const LIFT: f32 = 0.015;
/// How far a screw stays clear of the device on its way to the mat.
const SCREW_CLEAR: f32 = 0.01;

/// The cover of a device, apart from the other parts that move.
type CoverOnly = (With<Cover>, Without<Casing>, Without<Aside>);
type AsideOnly = (Without<Casing>, Without<Cover>);

/// A screw that the screwdriver turns.
#[derive(Component, Debug)]
pub struct Turning {
    /// The middle of the outer face of the head, with +Y into the device.
    pub at: Transform,
    /// How far the screw has turned around its axis, in radians.
    pub angle: f32,
}

impl Casing {
    /// Moves the cover and the screws between the device and the mat, as
    /// the casing is.
    pub(super) fn travel(
        time: Res<Time>,
        mut casings: Query<(&mut Casing, &Transform, &Holdable, &Children)>,
        mut covers: Query<&mut Transform, CoverOnly>,
        mut asides: Query<(Entity, &mut Aside, &mut Transform, Has<Turning>), AsideOnly>,
        mut commands: Commands,
    ) {
        let dt = time.delta_secs();
        for (mut casing, device, holdable, children) in &mut casings {
            let goal = if casing.open { 1.0 } else { 0.0 };
            let progress = casing
                .cover
                .map_or(goal, |progress| towards(progress, goal, dt / COVER_TIME));
            if casing.cover != Some(progress) {
                casing.cover = Some(progress);
            }
            let around = Around {
                device: *device,
                bounds: casing.bounds,
            };
            let mat = Self::cover_at(&casing.bounds, holdable.rest());
            let cover = cover_pose(progress, &around, mat);
            let local = Transform::from_matrix(Mat4::from(
                device.compute_affine().inverse() * cover.compute_affine(),
            ));
            for &child in children {
                if let Ok(mut transform) = covers.get_mut(child) {
                    transform.set_if_neq(local);
                }
            }
            for (index, &screw) in casing.aside.iter().enumerate() {
                let Ok((entity, mut aside, mut transform, turned)) = asides.get_mut(screw) else {
                    continue;
                };
                let goal = if casing.loose[index] { 1.0 } else { 0.0 };
                let before = aside.progress;
                let progress = towards(before, goal, dt / SCREW_TIME);
                if progress != before {
                    aside.progress = progress;
                }
                // The screw starts to turn, out of its hole or into it.
                if before <= TURN_FROM && aside.progress > TURN_FROM {
                    commands.trigger(Play(Sound::ScrewOut));
                } else if before >= TURN_TO && aside.progress < TURN_TO {
                    commands.trigger(Play(Sound::ScrewIn));
                }
                let hole = cover * Transform::from_translation(casing.holes[index]);
                let mat = Self::aside_at(&casing.bounds, index, holdable.rest());
                let (pose, turning) = screw_pose(aside.progress, hole, &around, mat);
                transform.set_if_neq(pose);
                match turning {
                    Some(turning) => {
                        commands.entity(entity).insert(turning);
                    }
                    None if turned => {
                        commands.entity(entity).remove::<Turning>();
                    }
                    None => {}
                }
            }
        }
    }
}

/// The device that the screws and the cover travel around.
struct Around {
    device: Transform,
    bounds: Block,
}

impl Around {
    /// `t` of the way from `from` to `to`, swinging around the middle of the
    /// device so that it stays `clear` of it on the way.
    fn carry(&self, from: Transform, to: Transform, t: f32, clear: f32) -> Transform {
        let t = smooth(t);
        let middle = self.device.transform_point(self.bounds.center);
        let (start, end) = (from.translation - middle, to.translation - middle);
        let (a, b) = (start.normalize_or(Vec3::Y), end.normalize_or(Vec3::Y));
        let direction = Quat::IDENTITY.slerp(Quat::from_rotation_arc(a, b), t) * a;
        let distance = start
            .length()
            .lerp(end.length(), t)
            .max(self.reach(direction) + clear * (PI * t).sin());
        Transform::from_translation(middle + direction * distance)
            .with_rotation(from.rotation.slerp(to.rotation, t))
    }

    /// How far the box of the device reaches from its middle along
    /// `direction`.
    fn reach(&self, direction: Vec3) -> f32 {
        let local = self.device.rotation.inverse() * direction;
        let half = self.bounds.size / 2.0;
        (0..3)
            .map(|axis| half[axis] / local[axis].abs())
            .fold(f32::INFINITY, f32::min)
    }
}

/// Where a screw is at `progress`, from `hole` in the device at 0 to `mat`
/// at 1, and how the screwdriver turns it.
fn screw_pose(
    progress: f32,
    hole: Transform,
    around: &Around,
    mat: Transform,
) -> (Transform, Option<Turning>) {
    let turned = ((progress - TURN_FROM) / (TURN_TO - TURN_FROM)).clamp(0.0, 1.0);
    let out = hole * Transform::from_translation(Vec3::NEG_Y * Thread::SIZE.y * turned);
    let angle = -turned * TURNS * TAU;
    let spun = out * Transform::from_rotation(Quat::from_rotation_y(angle));
    let pose = around.carry(
        spun,
        mat,
        ((progress - LET_GO) / (1.0 - LET_GO)).clamp(0.0, 1.0),
        SCREW_CLEAR,
    );
    let turning = (progress > 0.0 && progress < LET_GO).then(|| Turning {
        at: out * Transform::from_translation(Vec3::NEG_Y * SCREW.y),
        angle,
    });
    (pose, turning)
}

/// Where the cover is at `progress`, from on the device at 0 to `mat` at 1.
fn cover_pose(progress: f32, around: &Around, mat: Transform) -> Transform {
    let lifted = smooth((progress / LIFT_TO).clamp(0.0, 1.0));
    let off = around.device * Transform::from_translation(Vec3::NEG_Y * LIFT * lifted);
    // The plate is as wide as the device, so it swings out by half of it.
    let clear = around.bounds.size.xz().length() / 2.0;
    around.carry(
        off,
        mat,
        ((progress - LIFT_TO) / (1.0 - LIFT_TO)).clamp(0.0, 1.0),
        clear,
    )
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn towards(progress: f32, goal: f32, step: f32) -> f32 {
    progress + (goal - progress).clamp(-step, step)
}

#[cfg(test)]
mod tests {
    use super::super::COVER;
    use super::*;

    fn around(device: Transform) -> Around {
        Around {
            device,
            bounds: Block::new(Vec3::new(0.06, 0.02, 0.1)),
        }
    }

    /// Whether `point` is inside the box of the device.
    fn inside(around: &Around, point: Vec3) -> bool {
        let local = around.device.rotation.inverse() * (point - around.device.translation)
            - around.bounds.center;
        (local.abs() - around.bounds.size / 2.0).max_element() < 0.0
    }

    #[test]
    fn a_screw_turns_out_of_its_hole_and_lands_on_the_mat() {
        let around = around(Transform::from_xyz(0.0, 1.2, 0.0));
        let hole = Transform::from_xyz(0.01, 1.2, 0.02);
        let mat = Transform::from_xyz(0.3, 1.0, 0.1);
        let (start, turning) = screw_pose(0.0, hole, &around, mat);
        assert!(start.translation.distance(hole.translation) < 1e-5);
        assert!(turning.is_none());
        let (_, turning) = screw_pose(0.4, hole, &around, mat);
        assert!(turning.is_some());
        let (out, _) = screw_pose(LET_GO, hole, &around, mat);
        assert!((out.translation - hole.translation + Vec3::Y * Thread::SIZE.y).length() < 1e-5);
        let (end, turning) = screw_pose(1.0, hole, &around, mat);
        assert!(end.translation.distance(mat.translation) < 1e-5);
        assert!(end.rotation.angle_between(mat.rotation) < 1e-3);
        assert!(turning.is_none());
    }

    #[test]
    fn the_cover_lifts_off_before_it_moves_to_the_mat() {
        let around = around(Transform::from_xyz(0.0, 1.2, 0.0));
        let mat = Transform::from_xyz(0.2, 1.0, 0.0);
        let lifted = cover_pose(LIFT_TO, &around, mat);
        assert!((lifted.translation - around.device.translation + Vec3::Y * LIFT).length() < 1e-5);
        let end = cover_pose(1.0, &around, mat);
        assert!(end.translation.distance(mat.translation) < 1e-5);
    }

    #[test]
    fn screws_and_the_cover_go_around_the_device() {
        let mat = Transform::from_xyz(-0.12, 1.0, 0.0);
        for step in 0..12 {
            let turn = Quat::from_euler(
                EulerRot::XYZ,
                step as f32 * 0.9,
                step as f32 * 1.7,
                step as f32 * 0.4,
            );
            let around = around(Transform::from_xyz(0.0, 1.1, 0.0).with_rotation(turn));
            let hole = around.device * Transform::from_xyz(0.02, 0.0, 0.04);
            for index in 0..=100 {
                let progress = index as f32 / 100.0;
                let (screw, _) = screw_pose(progress, hole, &around, mat);
                let cover = cover_pose(progress, &around, mat);
                // Past the hole, which is on the face of the box.
                if progress > TURN_FROM {
                    assert!(
                        !inside(&around, screw.translation),
                        "turn {step} at {progress}"
                    );
                }
                assert!(
                    !inside(&around, cover.translation),
                    "turn {step} at {progress}"
                );
            }
        }
    }

    #[test]
    fn the_cover_lies_on_the_mat_with_its_outside_up() {
        let rest = Transform::from_xyz(0.0, 1.0, 0.0);
        let mat = Casing::cover_at(&Block::new(Vec3::new(0.06, 0.02, 0.1)), &rest);
        assert!((mat.rotation * Vec3::NEG_Y).y > 0.99);
        assert!(mat.translation.x < -0.06);
        assert!((mat.translation.y - Casing::ground(mat.translation) - COVER).abs() < 1e-6);
    }
}
