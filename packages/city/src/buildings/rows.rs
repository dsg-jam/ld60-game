use std::ops::Range;

use bevy::prelude::*;
use ld60_core::layout::shop;
use ld60_geometry::block::Block;
use rand::{Rng, RngExt};

use super::super::market::MARKET_HEIGHT;
use super::super::{EYE_HEIGHT, FACADES, HALF_LENGTH};
use super::PARAPET;

/// Plots are at least this wide, so everything on them fits.
const MIN_PLOT: f32 = 3.0;

pub(super) const ROWS: [Row; 6] = [
    // Our side of the street.
    Row {
        front: shop::FRONT,
        facing: -1.0,
        gap: Some(shop::SIZE.x / 2.0),
        half_length: HALF_LENGTH,
        width: 6.0..18.0,
        depth: 12.0..22.0,
        height: 10.0..65.0,
        skew: 3,
        setback: 0.0,
        alley_chance: 0.2,
        alleys: 1.2..3.0,
        market: true,
        additions: true,
        low_across: None,
        tiers: 0.3,
        crowns: 0.0,
    },
    // Across the street.
    Row {
        front: FACADES,
        facing: 1.0,
        gap: None,
        half_length: HALF_LENGTH,
        width: 6.0..18.0,
        depth: 12.0..22.0,
        height: 10.0..65.0,
        skew: 3,
        setback: 1.5,
        alley_chance: 0.35,
        alleys: 1.2..3.0,
        market: true,
        additions: true,
        low_across: Some(LOW_ACROSS),
        tiers: 0.3,
        crowns: 0.0,
    },
    // The skyline behind: towers of all heights with gaps between them.
    Row {
        front: -48.0,
        facing: 1.0,
        gap: None,
        half_length: 160.0,
        width: 8.0..22.0,
        depth: 15.0..30.0,
        height: 20.0..150.0,
        skew: 3,
        setback: 10.0,
        alley_chance: 0.55,
        alleys: 3.0..12.0,
        market: false,
        additions: true,
        low_across: None,
        tiers: 0.6,
        crowns: 0.15,
    },
    Row {
        front: -120.0,
        facing: 1.0,
        gap: None,
        half_length: 320.0,
        width: 12.0..35.0,
        depth: 25.0..45.0,
        height: 50.0..330.0,
        skew: 3,
        setback: 25.0,
        alley_chance: 0.5,
        alleys: 8.0..30.0,
        market: false,
        additions: false,
        low_across: None,
        tiers: 0.7,
        crowns: 0.25,
    },
    // Skyscrapers far apart, standing out against the low rows.
    Row {
        front: -200.0,
        facing: 1.0,
        gap: None,
        half_length: 300.0,
        width: 16.0..30.0,
        depth: 16.0..30.0,
        height: 180.0..380.0,
        skew: 1,
        setback: 20.0,
        alley_chance: 1.0,
        alleys: 25.0..80.0,
        market: false,
        additions: false,
        low_across: None,
        tiers: 0.8,
        crowns: 0.5,
    },
    // Silhouettes far away in the haze.
    Row {
        front: -260.0,
        facing: 1.0,
        gap: None,
        half_length: 600.0,
        width: 25.0..70.0,
        depth: 40.0..70.0,
        height: 120.0..500.0,
        skew: 3,
        setback: 40.0,
        alley_chance: 0.5,
        alleys: 20.0..60.0,
        market: false,
        additions: false,
        low_across: None,
        tiers: 0.7,
        crowns: 0.3,
    },
];

/// The buildings across from the shop within this distance of it stay low.
const LOW_HALF_WIDTH: f32 = 8.0;
/// Their height: the market floor and one more above it.
const LOW_ACROSS: Range<f32> = 6.5..7.5;
/// Straight across from the shop, the rows behind the street stay below the
/// buildings across, so the sky above them is open. Half the width of that
/// view and its top, as angles from the player's eyes in radians.
pub(super) const VIEW_CONE: f32 = 0.42;
const VIEW_TOP: f32 = 0.49;

const FLOOR_HEIGHT: Range<f32> = 3.0..3.8;

/// Where a building stands, and the height of its floors.
pub(super) struct Plot {
    pub(super) block: Block,
    pub(super) floor: f32,
    /// Kept low for the view from the shop, without anything on top.
    pub(super) low: bool,
}

/// One row of buildings, parallel to the street.
pub(super) struct Row {
    /// Z of the front faces.
    pub(super) front: f32,
    /// Direction along Z that the front faces look.
    pub(super) facing: f32,
    /// Half width of a gap around X = 0 for the shop.
    gap: Option<f32>,
    half_length: f32,
    width: Range<f32>,
    depth: Range<f32>,
    height: Range<f32>,
    /// Higher makes tall buildings rarer.
    skew: i32,
    /// How far a building may stand back from the row front.
    setback: f32,
    alley_chance: f64,
    alleys: Range<f32>,
    /// Whether the ground floor is a market.
    pub(super) market: bool,
    /// Whether people built additions onto the front.
    pub(super) additions: bool,
    /// Chances of a narrower tier on top, and of a glowing crown.
    pub(super) tiers: f64,
    pub(super) crowns: f64,
    /// Heights of the buildings across from the shop, kept low so the
    /// skyline shows behind them.
    low_across: Option<Range<f32>>,
}

impl Row {
    pub(super) fn lay_out(&self, rng: &mut impl Rng) -> Vec<Plot> {
        let mut plots = Vec::new();
        let mut x = -self.half_length;
        while x < self.half_length {
            let mut width = rng.random_range(self.width.clone());
            if let Some(gap) = self.gap
                && x < gap
                && x + width > -gap
            {
                // The building ends at the gap, the next one starts after.
                // A sliver left before the gap stays empty.
                if x < -gap - MIN_PLOT {
                    width = -gap - x;
                } else {
                    x = gap;
                    continue;
                }
            }
            let depth = rng.random_range(self.depth.clone());
            let front = self.front - self.facing * rng.random_range(0.0..=self.setback);
            let mut height = self.height.start
                + (self.height.end - self.height.start) * rng.random::<f32>().powi(self.skew);
            if let Some(low) = &self.low_across
                && x < LOW_HALF_WIDTH
                && x + width > -LOW_HALF_WIDTH
            {
                height = rng.random_range(low.clone());
            }
            let distance = shop::FRONT - front;
            let view = distance * VIEW_CONE.tan();
            let low = !self.market && x < view && x + width > -view;
            if low {
                height = height.min(EYE_HEIGHT + distance * VIEW_TOP.tan());
            }
            // Whole floors, and a parapet on top.
            let floor = rng.random_range(FLOOR_HEIGHT);
            let base = if self.market { MARKET_HEIGHT } else { 0.0 };
            let floors = if low {
                ((height - base - PARAPET) / floor).floor().max(1.0)
            } else {
                ((height - base) / floor).round().max(1.0)
            };
            let height = base + floors * floor + PARAPET;
            plots.push(Plot {
                block: Block::between(
                    Vec3::new(x, 0.0, front - self.facing * depth),
                    Vec3::new(x + width, height, front),
                ),
                floor,
                low,
            });
            x += width;
            if rng.random_bool(self.alley_chance) {
                x += rng.random_range(self.alleys.clone());
            }
        }
        plots
    }
}
