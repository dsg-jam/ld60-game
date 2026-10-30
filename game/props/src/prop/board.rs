use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use rand::{Rng, RngExt};

use super::{Prop, part, shade};

const MASKS: [Color; 3] = [
    Color::srgb_u8(0x2a, 0x5a, 0x34),
    Color::srgb_u8(0x24, 0x4a, 0x4a),
    Color::srgb_u8(0x3a, 0x4e, 0x2a),
];
const CHIP: Color = Color::srgb_u8(0x1c, 0x1c, 0x1e);
const CAPACITORS: [Color; 2] = [
    Color::srgb_u8(0x2a, 0x3a, 0x7a),
    Color::srgb_u8(0x1a, 0x1a, 0x1c),
];
const THICKNESS: f32 = 0.0016;
const CHIP_HEIGHT: f32 = 0.003;
const CAPACITOR: Vec3 = Vec3::new(0.008, 0.012, 0.008);

/// A bare circuit board, pulled from a dead device.
pub(crate) struct Board {
    size: Vec2,
    mask: Color,
    /// Chips and capacitors, as boxes on the board. Capacitors are taller
    /// than wide.
    parts: Vec<Block>,
}

impl Board {
    /// A board within `most`, with parts on it if `parts`, or none if it
    /// does not fit.
    pub(crate) fn random(rng: &mut impl Rng, most: Vec3, parts: bool) -> Option<Self> {
        let size = Vec2::new(
            rng.random_range(0.07..0.15f32).min(most.x),
            rng.random_range(0.05..0.11f32).min(most.z),
        );
        if size.cmplt(Vec2::splat(0.04)).any() || most.y < THICKNESS + CAPACITOR.y {
            return None;
        }
        let mut blocks = Vec::new();
        if parts {
            let half = size / 2.0 - 0.012;
            for _ in 0..rng.random_range(1..=4) {
                let chip = Vec3::new(
                    rng.random_range(0.01..0.035),
                    CHIP_HEIGHT,
                    rng.random_range(0.008..0.015),
                );
                let at = Vec2::new(
                    rng.random_range(-half.x..half.x),
                    rng.random_range(-half.y..half.y),
                );
                blocks.push(Block::new(chip).translated(Vec3::new(at.x, THICKNESS, at.y)));
            }
            for _ in 0..rng.random_range(0..=3) {
                let at = Vec2::new(
                    rng.random_range(-half.x..half.x),
                    rng.random_range(-half.y..half.y),
                );
                blocks.push(Block::new(CAPACITOR).translated(Vec3::new(at.x, THICKNESS, at.y)));
            }
        }
        Some(Self {
            size,
            mask: shade(rng, &MASKS),
            parts: blocks,
        })
    }
}

impl Prop for Board {
    fn size(&self) -> Vec3 {
        let height = self
            .parts
            .iter()
            .map(|part| part.max().y)
            .fold(THICKNESS, f32::max);
        Vec3::new(self.size.x, height, self.size.y)
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        let board = Block::new(Vec3::new(self.size.x, THICKNESS, self.size.y));
        part(parent, board, self.mask, Finish::Plastic);
        for (index, block) in self.parts.iter().enumerate() {
            if block.size.y > CHIP_HEIGHT {
                let color = CAPACITORS[index % CAPACITORS.len()];
                parent.spawn((Shape::cylinder(block, color), Finish::Plastic));
            } else {
                part(parent, *block, CHIP, Finish::Plastic);
            }
        }
    }
}
