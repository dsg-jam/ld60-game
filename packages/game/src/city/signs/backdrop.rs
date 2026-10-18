use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::{GLYPHS, NEON, PANEL_DEPTH, Sign};
use crate::city::buildings::{Backdrop, FacadeMaterial};
use crate::parts::script::Word;
use crate::workshop::shop;

const CHANCE: f64 = 0.5;
const IN_SIGHT_CHANCE: f64 = 0.9;
const ROOFTOP_CHANCE: f64 = 0.5;
/// Tall signs running down the front of a building.
const TALL_WIDTH: Range<f32> = 5.0..9.0;
/// How far below the roof they start.
const TALL_DROP: Range<f32> = 2.0..6.0;
/// Wide signs standing on a roof, on legs.
const ROOFTOP_HEIGHT: Range<f32> = 3.0..8.0;
const ROOFTOP_WIDTH: Range<f32> = 0.6..0.9;
const LEGS: f32 = 1.2;
/// Seen from the shop, the roofs across the street reach up to about this
/// angle, in radians.
const OVER_ROOFS: f32 = 0.42;
const EYE_HEIGHT: f32 = 1.5;

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
        let color = NEON[rng.random_range(0..NEON.len())];
        if let Some(roof) = backdrop.roof
            && rng.random_bool(ROOFTOP_CHANCE)
        {
            let word = Word::random(rng, GLYPHS);
            let size = Sign::fit(
                &word,
                rng.random_range(ROOFTOP_HEIGHT),
                roof.size.x * rng.random_range(ROOFTOP_WIDTH),
            );
            let center = Vec3::new(
                roof.center.x,
                roof.max().y + LEGS + size.y / 2.0,
                roof.max().z - 1.0,
            );
            return Some(Sign {
                transform: Transform::from_translation(center),
                size,
                color,
                double_sided: false,
                vertical: false,
                spill: false,
                framed: rng.random_bool(0.5),
                legs: LEGS,
                word,
            });
        }
        let word = Word::random(rng, GLYPHS);
        let width = rng.random_range(TALL_WIDTH);
        let height = Sign::fit_vertical(&word, width);
        if height > building.size.y * 0.7 {
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
            // Towards the side, but never past the edge of the building.
            building.center.x + side * (building.size.x / 2.0 - width / 2.0 - 1.0).max(0.0),
            y,
            building.max().z + PANEL_DEPTH / 2.0,
        );
        facade.extension.cover(Rect::from_center_size(
            center.xy(),
            Vec2::new(width, height),
        ));
        Some(Sign {
            transform: Transform::from_translation(center),
            size: Vec2::new(width, height),
            color,
            double_sided: false,
            vertical: true,
            spill: false,
            framed: true,
            legs: 0.0,
            word,
        })
    }
}
