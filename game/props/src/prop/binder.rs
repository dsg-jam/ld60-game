use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use rand::{Rng, RngExt};

use super::{Prop, label, part, placed, shade};

/// Cloth like the file of manuals on the counter, and others.
const CLOTHS: [Color; 6] = [
    Color::srgb_u8(0x2a, 0x3c, 0x5e),
    Color::srgb_u8(0x7a, 0x2a, 0x26),
    Color::srgb_u8(0x34, 0x50, 0x36),
    Color::srgb_u8(0x26, 0x26, 0x28),
    Color::srgb_u8(0x6e, 0x6c, 0x66),
    Color::srgb_u8(0x9a, 0x7a, 0x34),
];
const PAGES: Color = Color::srgb_u8(0xd8, 0xd0, 0xbc);
const HOLE: Color = Color::srgb_u8(0x0c, 0x0c, 0x0e);
const METAL: Color = Color::srgb_u8(0xa0, 0xa4, 0xa8);
const BOARD: f32 = 0.004;
/// Radius of the rounded corners at the back edge of a cover.
const CORNER: f32 = 0.006;
/// The pages stop this far short of the edges of the covers.
const INSET: f32 = 0.008;
const FINGER_HOLE_RADIUS: f32 = 0.012;
/// Thickness of the metal grommet around the finger hole.
const GROMMET_WIRE: f32 = 0.0016;
/// Height of the finger hole above the bottom edge.
const FINGER_HOLE_Y: f32 = 0.045;

/// A cloth covered ring binder standing shut, its spine to the front.
pub(crate) struct Binder {
    size: Vec3,
    cloth: Color,
    /// Thin files have no finger hole in the spine.
    hole: bool,
    writing: u32,
}

impl Binder {
    /// A binder `height` tall, within `most`, or none if it does not fit.
    pub(crate) fn random(rng: &mut impl Rng, height: f32, most: Vec3) -> Option<Self> {
        let size = Vec3::new(
            rng.random_range(0.03..0.085f32).min(most.x),
            height,
            rng.random_range(0.2..0.25f32).min(most.z),
        );
        (height <= most.y && size.x > 0.02 && size.z > 0.18).then(|| Self {
            size,
            cloth: shade(rng, &CLOTHS),
            hole: size.x > 0.045,
            writing: rng.random(),
        })
    }

    /// A height that fits under `most`, for a run of binders.
    pub(crate) fn height(rng: &mut impl Rng, most: f32) -> Option<f32> {
        (most >= 0.24).then(|| rng.random_range(0.24..=most.min(0.3)))
    }

    /// The covers, the spine and the cloth bent round between them.
    fn spawn_cloth(&self, parent: &mut ChildSpawnerCommands) {
        let size = self.size;
        let front = size.z / 2.0 - BOARD;
        let inner = size.x / 2.0 - BOARD;
        for side in [-1.0, 1.0] {
            // The bound edge of the board at the spine, its back edge round.
            let cover = Mat3::from_cols(Vec3::NEG_Z, Vec3::X * -side, Vec3::Y * side);
            parent.spawn((
                Shape::board(Vec3::new(size.z - BOARD, BOARD, size.y), CORNER, self.cloth),
                Finish::Fabric,
                Transform::from_xyz(side * size.x / 2.0, size.y / 2.0, front)
                    .with_rotation(Quat::from_mat3(&cover)),
            ));
            let around = if side > 0.0 {
                (-FRAC_PI_2, 0.0)
            } else {
                (PI, PI + FRAC_PI_2)
            };
            parent.spawn((
                Shape::bend(around, BOARD, size.y, self.cloth),
                Finish::Fabric,
                Transform::from_xyz(side * inner, size.y / 2.0, front).with_rotation(
                    Quat::from_mat3(&Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y)),
                ),
            ));
        }
        let spine = Block::new(Vec3::new(2.0 * inner, size.y, BOARD))
            .translated(Vec3::Z * (front + BOARD / 2.0));
        part(parent, spine, self.cloth, Finish::Fabric);
    }

    /// A metal grommet around a hole that looks into the dark inside.
    fn spawn_finger_hole(parent: &mut ChildSpawnerCommands, at: Vec3) {
        let facing =
            Transform::from_translation(at).with_rotation(Quat::from_rotation_x(FRAC_PI_2));
        placed(parent, facing, |hole| {
            let dark = Block::new(Vec3::new(
                2.0 * FINGER_HOLE_RADIUS,
                0.0004,
                2.0 * FINGER_HOLE_RADIUS,
            ));
            hole.spawn(Shape::cylinder(&dark, HOLE));
            let outer = 2.0 * (FINGER_HOLE_RADIUS + GROMMET_WIRE);
            let grommet = Block::new(Vec3::new(outer, GROMMET_WIRE, outer));
            hole.spawn((Shape::ring(&grommet, METAL), Finish::Metal));
        });
    }
}

impl Prop for Binder {
    fn size(&self) -> Vec3 {
        self.size
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        let size = self.size;
        self.spawn_cloth(parent);
        let half = size / 2.0 - BOARD;
        let pages = Block::between(
            Vec3::new(-half.x, INSET, -size.z / 2.0 + INSET),
            Vec3::new(half.x, size.y - INSET, half.z - INSET / 4.0),
        );
        part(parent, pages, PAGES, Finish::Paper);
        let front = size.z / 2.0;
        let width = (size.x * 0.7).min(0.04);
        label(
            parent,
            Vec3::new(0.0, size.y * 0.66, front),
            Vec2::new(width, size.y * 0.32),
            self.writing,
        );
        if self.hole {
            Self::spawn_finger_hole(parent, Vec3::new(0.0, FINGER_HOLE_Y, front));
        }
    }
}
