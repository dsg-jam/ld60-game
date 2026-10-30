use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use rand::{Rng, RngExt};

use super::{Prop, label, part, placed, shade};

const COVERS: [Color; 6] = [
    Color::srgb_u8(0x8a, 0x2e, 0x28),
    Color::srgb_u8(0x2e, 0x46, 0x6e),
    Color::srgb_u8(0x3a, 0x5a, 0x3a),
    Color::srgb_u8(0x26, 0x26, 0x28),
    Color::srgb_u8(0x7a, 0x7a, 0x76),
    Color::srgb_u8(0xb0, 0x8a, 0x30),
];
const PAGES: Color = Color::srgb_u8(0xd8, 0xd0, 0xbc);
const HOLE: Color = Color::srgb_u8(0x14, 0x12, 0x10);
const COVER: f32 = 0.003;
/// The pages stop this far short of the edges of the cover.
const INSET: f32 = 0.008;
const HOLE_SIZE: f32 = 0.022;

/// A ring binder standing on its end, its spine to the front.
pub(crate) struct Binder {
    size: Vec3,
    cover: Color,
    /// Thin files and magazines have no finger hole in the spine.
    hole: bool,
}

impl Binder {
    /// A binder `height` tall, within `most`, or none if it does not fit.
    pub(crate) fn random(rng: &mut impl Rng, height: f32, most: Vec3) -> Option<Self> {
        let size = Vec3::new(
            rng.random_range(0.03..0.085f32).min(most.x),
            height,
            rng.random_range(0.24..0.29f32).min(most.z),
        );
        (height <= most.y && size.x > 0.02 && size.z > 0.2).then(|| Self {
            size,
            cover: shade(rng, &COVERS),
            hole: size.x > 0.045,
        })
    }

    /// A height that fits under `most`, for a run of binders.
    pub(crate) fn height(rng: &mut impl Rng, most: f32) -> Option<f32> {
        (most >= 0.24).then(|| rng.random_range(0.24..=most.min(0.32)))
    }
}

impl Prop for Binder {
    fn size(&self) -> Vec3 {
        self.size
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        let size = self.size;
        for side in [-1.0, 1.0] {
            let cover =
                Block::new(size.with_x(COVER)).translated(Vec3::X * side * (size.x - COVER) / 2.0);
            part(parent, cover, self.cover, Finish::Plastic);
        }
        let spine = Block::new(Vec3::new(size.x - 2.0 * COVER, size.y, COVER))
            .translated(Vec3::Z * (size.z - COVER) / 2.0);
        part(parent, spine, self.cover, Finish::Plastic);
        let pages = Block::new(Vec3::new(
            size.x - 2.0 * COVER,
            size.y - 2.0 * INSET,
            size.z - INSET - COVER,
        ))
        .translated(Vec3::new(0.0, INSET, (INSET - COVER) / 2.0));
        part(parent, pages, PAGES, Finish::Paper);
        let front = size.z / 2.0;
        let width = (size.x * 0.7).min(0.04);
        label(
            parent,
            Vec3::new(0.0, size.y * 0.7, front),
            Vec2::new(width, size.y * 0.3),
            3,
        );
        if self.hole {
            let at = Transform::from_xyz(0.0, size.y * 0.2, front)
                .with_rotation(Quat::from_rotation_x(FRAC_PI_2));
            placed(parent, at, |hole| {
                let block = Block::new(Vec3::new(HOLE_SIZE, 0.001, HOLE_SIZE));
                hole.spawn(Shape::cylinder(&block, HOLE));
            });
        }
    }
}
