use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::prop::{Prop, placed};

/// Props are turned by up to this much in a pile, in radians.
const PILE_TURN: f32 = 0.2;
/// And pushed off the middle of the one below by up to this much.
const PILE_SHIFT: f32 = 0.008;

/// Props placed together, as one prop that bounds them all.
#[derive(Default)]
pub(crate) struct Group {
    props: Vec<(Box<dyn Prop>, Transform)>,
    min: Vec3,
    max: Vec3,
}

impl Group {
    /// `props` stacked on top of each other, each turned and pushed a
    /// little.
    pub(crate) fn pile(rng: &mut impl Rng, props: impl IntoIterator<Item = Box<dyn Prop>>) -> Self {
        let mut group = Self::default();
        let mut height = 0.0;
        for prop in props {
            let shift = Vec2::splat(PILE_SHIFT);
            let shift = Vec2::new(
                rng.random_range(-shift.x..=shift.x),
                rng.random_range(-shift.y..=shift.y),
            );
            let at = Transform::from_xyz(shift.x, height, shift.y).with_rotation(
                Quat::from_rotation_y(rng.random_range(-PILE_TURN..PILE_TURN)),
            );
            height += prop.size().y;
            group.add(prop, at);
        }
        group
    }

    /// `props` side by side along X, `gap` apart, backs lined up.
    pub(crate) fn row(
        props: impl IntoIterator<Item = Box<dyn Prop>>,
        mut gap: impl FnMut() -> f32,
    ) -> Self {
        let mut group = Self::default();
        let mut x = 0.0;
        for prop in props {
            let size = prop.size();
            group.add(
                prop,
                Transform::from_xyz(x + size.x / 2.0, 0.0, size.z / 2.0),
            );
            x += size.x + gap();
        }
        group
    }

    /// Leans `prop` by `angle` to the left, onto the right side of the
    /// group, which must be at least as high as `prop`.
    pub(crate) fn lean(&mut self, prop: Box<dyn Prop>, angle: f32) {
        let size = prop.size();
        let turn = Quat::from_rotation_z(angle);
        // The top left corner rests on the group, the bottom left one on
        // the ground.
        let pivot = Vec3::new(self.max.x + size.y * angle.sin(), 0.0, size.z / 2.0);
        let at =
            Transform::from_translation(pivot + turn * Vec3::X * size.x / 2.0).with_rotation(turn);
        self.add(prop, at);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.props.is_empty()
    }

    fn add(&mut self, prop: Box<dyn Prop>, at: Transform) {
        let half = prop.size() / 2.0;
        let corners = (0..8).map(|corner| {
            let sign = Vec3::new(
                if corner & 1 == 0 { -1.0 } else { 1.0 },
                if corner & 2 == 0 { -1.0 } else { 1.0 },
                if corner & 4 == 0 { -1.0 } else { 1.0 },
            );
            at.transform_point(Vec3::Y * half.y + sign * half)
        });
        let (min, max) = corners.fold(
            if self.props.is_empty() {
                (Vec3::MAX, Vec3::MIN)
            } else {
                (self.min, self.max)
            },
            |(min, max), corner| (min.min(corner), max.max(corner)),
        );
        (self.min, self.max) = (min, max);
        self.props.push((prop, at));
    }
}

impl Prop for Group {
    fn size(&self) -> Vec3 {
        if self.props.is_empty() {
            Vec3::ZERO
        } else {
            self.max - self.min
        }
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        let middle = (self.min + self.max) / 2.0;
        let offset = -middle.with_y(self.min.y);
        for (prop, at) in &self.props {
            let at = Transform {
                translation: at.translation + offset,
                ..*at
            };
            placed(parent, at, |spot| prop.spawn(spot));
        }
    }
}
