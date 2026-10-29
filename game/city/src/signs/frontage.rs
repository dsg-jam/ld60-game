use std::f32::consts::FRAC_PI_2;
use std::ops::Range;

use bevy::prelude::*;
use ld60_game_core::parts::script::Word;
use ld60_geometry::block::Block;
use rand::{Rng, RngExt};

use super::{GLYPHS, NEON, PANEL_DEPTH, Sign};
use crate::buildings::FacadeMaterial;
use crate::market::{AWNING_ABOVE, MARKET_HEIGHT};

const SHOP_SIGN_CHANCE: f64 = 0.8;
const SHOP_SIGN_HEIGHT: Range<f32> = 0.9..1.6;
/// Shop signs hang above the stall awnings.
const SHOP_SIGN_BOTTOM: f32 = MARKET_HEIGHT + AWNING_ABOVE + 0.25;
const BLADE_CHANCE: f64 = 0.45;
const BLADE_REACH: f32 = 1.6;
/// Height of a blade sign per glyph, as a share of its reach.
const BLADE_GLYPH: f32 = 0.8;
const BLADE_BOTTOM: Range<f32> = 5.0..9.0;
/// Signs on a wall end this far below the roof.
const ROOF_MARGIN: f32 = 0.3;
/// Width of the wall strip behind the bracket of a blade sign.
const BLADE_BRACKET: f32 = 0.6;
const BILLBOARD_CHANCE: f64 = 0.25;
const BILLBOARD_HEIGHT: Range<f32> = 3.0..6.0;
/// Only tall buildings get billboards.
const BILLBOARD_BUILDING: f32 = 20.0;
/// Billboards hang this far below the roof.
const BILLBOARD_DROP: Range<f32> = 1.0..6.0;
/// Signs on a wall take up at most this share of its width.
const WALL_SHARE: f32 = 0.9;
/// Blade signs stick out this far in from the corner of the building.
const BLADE_INSET: f32 = 0.6;

impl Sign {
    /// Signs on the street side of a building along the street: a shop sign
    /// over the market, a blade sign sticking out, and a billboard on tall
    /// buildings, each only sometimes.
    pub(super) fn on_frontage(
        building: &Block,
        facade: &mut FacadeMaterial,
        rng: &mut impl Rng,
    ) -> Vec<Self> {
        let mut signs = Vec::new();
        if rng.random_bool(SHOP_SIGN_CHANCE) {
            signs.push(Self::shop_sign(building, facade, rng));
        }
        if rng.random_bool(BLADE_CHANCE) {
            signs.extend(Self::blade(building, facade, rng));
        }
        if building.size.y > BILLBOARD_BUILDING && rng.random_bool(BILLBOARD_CHANCE) {
            signs.push(Self::billboard(building, facade, rng));
        }
        signs
    }

    fn shop_sign(building: &Block, facade: &mut FacadeMaterial, rng: &mut impl Rng) -> Self {
        let word = Word::random(rng, GLYPHS);
        let size = Self::fit(
            &word,
            rng.random_range(SHOP_SIGN_HEIGHT),
            building.size.x * WALL_SHARE,
        );
        let center = Vec3::new(
            building.center.x,
            SHOP_SIGN_BOTTOM + size.y / 2.0,
            building.max().z + PANEL_DEPTH / 2.0,
        );
        facade
            .extension
            .cover(Rect::from_center_size(center.xy(), size));
        Self {
            transform: Transform::from_translation(center),
            size,
            neon: rng.random_range(0..NEON.len()),
            double_sided: false,
            vertical: false,
            spill: true,
            framed: rng.random_bool(0.5),
            legs: 0.0,
            word,
        }
    }

    fn blade(building: &Block, facade: &mut FacadeMaterial, rng: &mut impl Rng) -> Option<Self> {
        // Short enough to fit between the shop signs and the roof.
        let glyph = BLADE_REACH * BLADE_GLYPH;
        let room = building.max().y - ROOF_MARGIN - BLADE_BOTTOM.start;
        let word = Word::random(rng, GLYPHS).truncated((room / glyph) as usize);
        let size = Vec2::new(BLADE_REACH, glyph * word.len() as f32);
        let side = if rng.random_bool(0.5) { -1.0 } else { 1.0 };
        let bottom = rng
            .random_range(BLADE_BOTTOM)
            .min(building.max().y - ROOF_MARGIN - size.y);
        let neon = rng.random_range(0..NEON.len());
        if word.len() < GLYPHS.start {
            return None;
        }
        let center = Vec3::new(
            building.center.x + side * (building.size.x / 2.0 - BLADE_INSET),
            bottom + size.y / 2.0,
            building.max().z + size.x / 2.0,
        );
        // The bracket on the wall takes a strip of the facade.
        facade.extension.cover(Rect::from_center_size(
            center.xy(),
            Vec2::new(BLADE_BRACKET, size.y),
        ));
        Some(Self {
            transform: Transform::from_translation(center)
                .with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
            size,
            neon,
            double_sided: true,
            vertical: true,
            spill: false,
            framed: true,
            legs: 0.0,
            word,
        })
    }

    fn billboard(building: &Block, facade: &mut FacadeMaterial, rng: &mut impl Rng) -> Self {
        let word = Word::random(rng, GLYPHS);
        let size = Self::fit(
            &word,
            rng.random_range(BILLBOARD_HEIGHT),
            building.size.x * WALL_SHARE,
        );
        let center = Vec3::new(
            building.center.x,
            building.max().y - size.y / 2.0 - rng.random_range(BILLBOARD_DROP),
            building.max().z + PANEL_DEPTH / 2.0,
        );
        facade
            .extension
            .cover(Rect::from_center_size(center.xy(), size));
        Self {
            transform: Transform::from_translation(center),
            size,
            neon: rng.random_range(0..NEON.len()),
            double_sided: false,
            vertical: false,
            spill: false,
            framed: true,
            legs: 0.0,
            word,
        }
    }
}
