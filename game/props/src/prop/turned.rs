use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use super::{Prop, placed};

/// A prop turned onto its left side.
pub(crate) struct Lying<P>(pub(crate) P);

/// A prop stood up on its back end, its top to the front.
pub(crate) struct Standing<P>(pub(crate) P);

impl<P: Prop> Prop for Lying<P> {
    fn size(&self) -> Vec3 {
        let size = self.0.size();
        Vec3::new(size.y, size.x, size.z)
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        let size = self.0.size();
        let at = Transform::from_xyz(size.y / 2.0, size.x / 2.0, 0.0)
            .with_rotation(Quat::from_rotation_z(FRAC_PI_2));
        placed(parent, at, |turned| self.0.spawn(turned));
    }
}

impl<P: Prop> Prop for Standing<P> {
    fn size(&self) -> Vec3 {
        let size = self.0.size();
        Vec3::new(size.x, size.z, size.y)
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        let size = self.0.size();
        let at = Transform::from_xyz(0.0, size.z / 2.0, -size.y / 2.0)
            .with_rotation(Quat::from_rotation_x(FRAC_PI_2));
        placed(parent, at, |turned| self.0.spawn(turned));
    }
}
