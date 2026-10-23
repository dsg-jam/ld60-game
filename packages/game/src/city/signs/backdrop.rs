use std::ops::Range;

use bevy::prelude::*;
use ld60_core::layout::shop;
use ld60_core::parts::script::Word;
use rand::{Rng, RngExt};

use super::{GLYPHS, NEON, PANEL_DEPTH, Sign};
use crate::city::EYE_HEIGHT;
use crate::city::buildings::{Backdrop, FacadeMaterial};

const CHANCE: f64 = 0.5;
const IN_SIGHT_CHANCE: f64 = 0.9;
const ROOFTOP_CHANCE: f64 = 0.5;
/// Tall signs running down the front of a building.
const TALL_WIDTH: Range<f32> = 5.0..9.0;
/// How far below the roof they start.
const TALL_DROP: Range<f32> = 2.0..6.0;
/// They keep this far from the sides of the building.
const TALL_MARGIN: f32 = 1.0;
/// And take up at most this share of its height.
const TALL_SHARE: f32 = 0.7;
/// Wide signs standing on a roof, on legs.
const ROOFTOP_HEIGHT: Range<f32> = 3.0..8.0;
const ROOFTOP_WIDTH: Range<f32> = 0.6..0.9;
const LEGS: f32 = 1.2;
/// Rooftop signs stand this far back from the front of the roof.
const ROOFTOP_SETBACK: f32 = 1.0;
/// Seen from the shop, the roofs across the street reach up to about this
/// angle, in radians.
const OVER_ROOFS: f32 = 0.42;

impl Sign {
    /// A large sign on a building behind the street, seen from afar, if it
    /// gets one.
    pub(super) fn on_backdrop(
        backdrop: &Backdrop,
        facade: &mut FacadeMaterial,
        rng: &mut impl Rng,
    ) -> Option<Self> {
        let chance = if backdrop.in_sight {
            IN_SIGHT_CHANCE
        } else {
            CHANCE
        };
        if !rng.random_bool(chance) {
            return None;
        }
        let building = &backdrop.front.block;
        let neon = rng.random_range(0..NEON.len());
        if let Some(roof) = backdrop.roof
            && rng.random_bool(ROOFTOP_CHANCE)
        {
            let word = Word::random(rng, GLYPHS);
            let size = Self::fit(
                &word,
                rng.random_range(ROOFTOP_HEIGHT),
                roof.size.x * rng.random_range(ROOFTOP_WIDTH),
            );
            let center = Vec3::new(
                roof.center.x,
                roof.max().y + LEGS + size.y / 2.0,
                roof.max().z - ROOFTOP_SETBACK,
            );
            return Some(Self {
                transform: Transform::from_translation(center),
                size,
                neon,
                double_sided: false,
                vertical: false,
                spill: false,
                framed: rng.random_bool(0.5),
                legs: LEGS,
                word,
            });
        }
        let word = Word::random(rng, GLYPHS);
        let width = rng
            .random_range(TALL_WIDTH)
            .min(building.size.x - 2.0 * TALL_MARGIN);
        let height = Self::fit_vertical(&word, width);
        if width <= 0.0 || height > building.size.y * TALL_SHARE {
            return None;
        }
        let mut side = if rng.random_bool(0.5) { -1.0 } else { 1.0 };
        let mut y = building.max().y - rng.random_range(TALL_DROP) - height / 2.0;
        if backdrop.in_sight {
            // Just over the roofs across the street, on the side towards the
            // gap between them.
            let distance = shop::FRONT - building.max().z;
            let bottom = EYE_HEIGHT + distance * OVER_ROOFS.tan();
            y = y.min(bottom + height / 2.0);
            side = -building.center.x.signum();
        }
        let center = Vec3::new(
            building.center.x + side * (building.size.x / 2.0 - width / 2.0 - TALL_MARGIN),
            y,
            building.max().z + PANEL_DEPTH / 2.0,
        );
        facade.extension.cover(Rect::from_center_size(
            center.xy(),
            Vec2::new(width, height),
        ));
        Some(Self {
            transform: Transform::from_translation(center),
            size: Vec2::new(width, height),
            neon,
            double_sided: false,
            vertical: true,
            spill: false,
            framed: true,
            legs: 0.0,
            word,
        })
    }
}
