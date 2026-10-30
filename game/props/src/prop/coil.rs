use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use rand::{Rng, RngExt};

use super::{Prop, shade};

const SHEATHS: [Color; 5] = [
    Color::srgb_u8(0x1e, 0x1e, 0x20),
    Color::srgb_u8(0x6e, 0x70, 0x72),
    Color::srgb_u8(0xc0, 0x5a, 0x22),
    Color::srgb_u8(0x2a, 0x4c, 0x8a),
    Color::srgb_u8(0xd8, 0xd4, 0xc8),
];
/// The turns of a coil lie side by side across this many cable widths.
const TURNS: std::ops::RangeInclusive<f32> = 1.5..=3.0;

/// A coil of cable lying flat.
pub(crate) struct Coil {
    size: Vec3,
    color: Color,
}

impl Coil {
    /// A coil within `most`, or none if it does not fit.
    pub(crate) fn random(rng: &mut impl Rng, most: Vec3) -> Option<Self> {
        let width = rng.random_range(0.1..0.22f32).min(most.x).min(most.z);
        let cable = rng.random_range(0.005..0.01) * rng.random_range(TURNS);
        (width >= 0.08 && cable <= most.y).then(|| Self {
            size: Vec3::new(width, cable, width),
            color: shade(rng, &SHEATHS),
        })
    }
}

impl Prop for Coil {
    fn size(&self) -> Vec3 {
        self.size
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        parent.spawn((
            Shape::ring(&Block::new(self.size), self.color),
            Finish::Grip,
        ));
    }
}
