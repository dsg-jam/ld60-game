use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use super::props::Props;
use ld60_core::player::camera::PlayerCamera;

use crate::device::Turning;

/// How fast the screwdriver moves to a screw and back.
const DECAY_RATE: f32 = 14.0;
/// The handle points at least this far away from the camera, in radians.
/// Along a screw that faces the player it would poke into the view.
const CLEAR: f32 = 0.9;

/// The screwdriver on the counter. It turns the screws of a device in and
/// out, see [`Turning`].
#[derive(Component, Debug)]
pub(super) struct Screwdriver {
    rest: Transform,
    /// Where the tip is, as the head of a screw it sits on. It eases between
    /// the screws and its rest.
    tip: Transform,
}

impl Screwdriver {
    pub(super) fn new(rest: Transform) -> Self {
        Self {
            rest,
            tip: rest * Self::tip_to_rest(),
        }
    }

    /// Puts the tip on the screw that turns, if any, and turns with it.
    /// The clutter stays at the origin, so its space is the world.
    pub(super) fn follow(
        time: Res<Time>,
        turning: Query<&Turning>,
        camera: Single<&Transform, (With<PlayerCamera>, Without<Screwdriver>)>,
        mut screwdrivers: Query<(&mut Screwdriver, &mut Transform)>,
    ) {
        let screw = turning.iter().next();
        let dt = time.delta_secs();
        for (mut screwdriver, mut transform) in &mut screwdrivers {
            let goal = screw.map_or(screwdriver.rest * Self::tip_to_rest(), |screw| {
                Self::clear_of(screw.at, camera.translation)
            });
            let tip = &mut screwdriver.tip;
            tip.translation
                .smooth_nudge(&goal.translation, DECAY_RATE, dt);
            tip.rotation.smooth_nudge(&goal.rotation, DECAY_RATE, dt);
            let angle = screw.map_or(0.0, |screw| screw.angle);
            transform.set_if_neq(
                screwdriver.tip
                    * Transform::from_rotation(Quat::from_rotation_y(angle))
                    * Self::rest_to_tip(),
            );
        }
    }

    /// `tip`, turned around the tip so the handle points at least
    /// [`CLEAR`] away from `camera`. It stays along the screw where it can.
    fn clear_of(tip: Transform, camera: Vec3) -> Transform {
        let handle = tip.rotation * Vec3::NEG_Y;
        let Some(to_camera) = (camera - tip.translation).try_normalize() else {
            return tip;
        };
        let off = handle.angle_between(to_camera);
        if off >= CLEAR {
            return tip;
        }
        // Straight at the camera, any way away will do, so lean up.
        let axis = to_camera
            .cross(handle)
            .try_normalize()
            .or_else(|| to_camera.cross(Vec3::Y).try_normalize())
            .unwrap_or(Vec3::X);
        tip.with_rotation(Quat::from_axis_angle(axis, CLEAR - off) * tip.rotation)
    }

    /// From the space of the screwdriver to the head of a screw: the tip on
    /// the middle of the head and the shaft along +Y, into the screw.
    fn rest_to_tip() -> Transform {
        let rotation = Quat::from_rotation_z(FRAC_PI_2);
        Transform::from_translation(-(rotation * Props::screwdriver_tip())).with_rotation(rotation)
    }

    /// The inverse of [`Self::rest_to_tip`].
    fn tip_to_rest() -> Transform {
        Transform::from_translation(Props::screwdriver_tip())
            .with_rotation(Quat::from_rotation_z(-FRAC_PI_2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_handle_leans_away_from_the_camera_only_when_it_points_there() {
        let camera = Vec3::new(0.0, 0.0, 0.3);
        // The screw faces the camera, so the handle would point at it.
        let facing = Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2));
        let leaned = Screwdriver::clear_of(facing, camera);
        let handle = leaned.rotation * Vec3::NEG_Y;
        assert!(handle.angle_between(camera) >= CLEAR - 1e-4, "{handle}");
        let sideways = Transform::from_rotation(Quat::from_rotation_z(FRAC_PI_2));
        assert_eq!(Screwdriver::clear_of(sideways, camera), sideways);
    }
}
