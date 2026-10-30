use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use rand::{Rng, RngExt};

use super::{LABEL, Prop, label, shade};

const LIDS: [Color; 4] = [
    Color::srgb_u8(0xa8, 0x2c, 0x24),
    Color::srgb_u8(0x22, 0x22, 0x24),
    Color::srgb_u8(0xd8, 0xd4, 0xc8),
    Color::srgb_u8(0x2a, 0x50, 0x8a),
];
/// Screws, nuts and washers, seen through the glass.
const HARDWARE: [Color; 2] = [
    Color::srgb_u8(0x8a, 0x8c, 0x90),
    Color::srgb_u8(0xa0, 0x88, 0x5a),
];
const TIN: Color = Color::srgb_u8(0x9a, 0x9c, 0xa0);
const PRINTS: [Color; 4] = [
    Color::srgb_u8(0xb8, 0x3a, 0x2a),
    Color::srgb_u8(0x2e, 0x5a, 0x40),
    Color::srgb_u8(0xd0, 0xa8, 0x3a),
    LABEL,
];
const LID: f32 = 0.014;
/// The lid stands out of the jar by this much all round.
const LID_OUT: f32 = 0.002;

/// A jar of small hardware with a screw lid and a label.
pub(crate) struct Jar {
    size: Vec3,
    lid: Color,
    contents: Color,
}

/// A tin can with a printed band around it.
pub(crate) struct Tin {
    size: Vec3,
    print: Color,
}

impl Jar {
    /// A jar within `most`, or none if it does not fit.
    pub(crate) fn random(rng: &mut impl Rng, most: Vec3) -> Option<Self> {
        let width = rng.random_range(0.06..0.1f32).min(most.x).min(most.z);
        let height = rng.random_range(0.08..0.14f32).min(most.y);
        (width >= 0.05 && height >= 0.07).then(|| Self {
            size: Vec3::new(width, height, width),
            lid: shade(rng, &LIDS),
            contents: shade(rng, &HARDWARE),
        })
    }
}

impl Tin {
    /// A tin within `most`, or none if it does not fit.
    pub(crate) fn random(rng: &mut impl Rng, most: Vec3) -> Option<Self> {
        let width = rng.random_range(0.07..0.11f32).min(most.x).min(most.z);
        let height = rng.random_range(0.06..0.15f32).min(most.y);
        (width >= 0.06 && height >= 0.05).then(|| Self {
            size: Vec3::new(width, height, width),
            print: shade(rng, &PRINTS),
        })
    }
}

impl Prop for Jar {
    fn size(&self) -> Vec3 {
        self.size
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        let glass = Block::new(self.size - Vec3::new(2.0 * LID_OUT, LID, 2.0 * LID_OUT));
        parent.spawn((Shape::cylinder(&glass, self.contents), Finish::Metal));
        let lid = Block::new(self.size.with_y(LID)).on_top_of(&glass);
        parent.spawn((Shape::cylinder(&lid, self.lid), Finish::Plastic));
        let width = glass.size.x * 0.45;
        label(
            parent,
            Vec3::new(0.0, glass.size.y * 0.5, glass.size.z / 2.0),
            Vec2::new(width, glass.size.y * 0.4),
            2,
        );
    }
}

impl Prop for Tin {
    fn size(&self) -> Vec3 {
        self.size
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        let can = Block::new(self.size - Vec3::new(0.002, 0.0, 0.002));
        parent.spawn((Shape::cylinder(&can, TIN), Finish::Metal));
        let band =
            Block::new(self.size.with_y(self.size.y * 0.6)).translated(Vec3::Y * self.size.y * 0.2);
        parent.spawn((Shape::cylinder(&band, self.print), Finish::Paper));
    }
}
