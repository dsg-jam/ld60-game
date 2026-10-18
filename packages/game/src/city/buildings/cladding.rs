use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::super::Pick;

const CONCRETE: [Color; 4] = [
    Color::srgb_u8(0x4a, 0x48, 0x46),
    Color::srgb_u8(0x40, 0x44, 0x48),
    Color::srgb_u8(0x52, 0x4c, 0x44),
    Color::srgb_u8(0x38, 0x3a, 0x40),
];
/// Faded paint: ochre, salmon, teal, cream, rust red, pale blue and mint.
const PAINT: [Color; 7] = [
    Color::srgb_u8(0x7a, 0x5c, 0x32),
    Color::srgb_u8(0x7a, 0x4a, 0x40),
    Color::srgb_u8(0x36, 0x56, 0x54),
    Color::srgb_u8(0x78, 0x72, 0x60),
    Color::srgb_u8(0x60, 0x32, 0x28),
    Color::srgb_u8(0x4c, 0x5a, 0x68),
    Color::srgb_u8(0x50, 0x68, 0x56),
];
const TILES: [Color; 4] = [
    Color::srgb_u8(0x7a, 0x7c, 0x78),
    Color::srgb_u8(0x4a, 0x54, 0x5e),
    Color::srgb_u8(0x5c, 0x44, 0x36),
    Color::srgb_u8(0x6a, 0x72, 0x5a),
];
const BRICK: [Color; 3] = [
    Color::srgb_u8(0x5e, 0x32, 0x24),
    Color::srgb_u8(0x6a, 0x40, 0x2c),
    Color::srgb_u8(0x4c, 0x2e, 0x24),
];
/// Some buildings are lighter or darker than their color.
const SHADE: Range<f32> = 0.8..1.15;

/// What the outer walls of a building are clad in.
///
/// Matches the constants in `walls.wesl`.
#[derive(Clone, Copy, Debug)]
pub(super) enum Cladding {
    Concrete = 0,
    Panels = 1,
    Tiles = 2,
    Brick = 3,
    Plaster = 4,
}

impl Cladding {
    pub(super) fn random(rng: &mut impl Rng) -> Self {
        match rng.random_range(0.0..1.0) {
            x if x < 0.25 => Self::Concrete,
            x if x < 0.45 => Self::Panels,
            x if x < 0.6 => Self::Tiles,
            x if x < 0.75 => Self::Brick,
            _ => Self::Plaster,
        }
    }

    /// A color for a building clad in this.
    pub(super) fn color(self, rng: &mut impl Rng) -> Color {
        let colors: &[Color] = match self {
            Self::Concrete | Self::Panels => &CONCRETE,
            Self::Tiles => &TILES,
            Self::Brick => &BRICK,
            Self::Plaster => &PAINT,
        };
        let color = *colors.pick(rng);
        let shade = rng.random_range(SHADE);
        let linear = color.to_linear();
        Color::linear_rgb(
            linear.red * shade,
            linear.green * shade,
            linear.blue * shade,
        )
    }
}
