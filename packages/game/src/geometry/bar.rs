use bevy::prelude::*;

/// A box stretched from one point to another, like a strut, a stroke of a
/// letter or a sloping sheet.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bar {
    pub(crate) from: Vec3,
    pub(crate) to: Vec3,
    /// Width, and thickness towards `up`.
    pub(crate) section: Vec2,
    /// The thickness points this way, as far as the length allows.
    pub(crate) up: Vec3,
}

impl Bar {
    /// A bar `thickness` wide and thick, turned any way about its length.
    pub(crate) fn square(from: Vec3, to: Vec3, thickness: f32) -> Self {
        let up = if (to - from).normalize_or_zero().y.abs() > 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        Self {
            from,
            to,
            section: Vec2::splat(thickness),
            up,
        }
    }

    /// The bar reaching `by` further past both ends, so joints close.
    pub(crate) fn extended(self, by: f32) -> Self {
        let along = (self.to - self.from).normalize_or_zero() * by;
        Self {
            from: self.from - along,
            to: self.to + along,
            ..self
        }
    }

    pub(crate) fn translated(self, offset: Vec3) -> Self {
        Self {
            from: self.from + offset,
            to: self.to + offset,
            ..self
        }
    }

    /// Width, thickness and length.
    pub(crate) fn size(&self) -> Vec3 {
        self.section.extend(self.from.distance(self.to))
    }

    /// Placement of the middle of the bar, its length along local Z.
    pub(crate) fn placement(&self) -> Transform {
        Transform::from_translation((self.from + self.to) / 2.0)
            .looking_to(self.to - self.from, self.up)
    }

    /// Placement of a unit cube scaled to fill the bar.
    pub(crate) fn unit_transform(&self) -> Transform {
        self.placement().with_scale(self.size())
    }
}
