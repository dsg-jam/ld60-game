//! A local shaking the rain out of its feathers.
//!
//! On every swing of the body the feathers flap and rub against each other,
//! a dry rustle that swells and fades many times a second. The water flies
//! off in drops that patter on the sidewalk around it.

use std::f32::consts::{PI, TAU};

use super::Samples;
use super::drops::{Bands, Dice, Ring, Rings};

/// Octave bands the feathers rustle in.
const RUSTLE: [f32; 4] = [1_000.0, 2_000.0, 4_000.0, 8_000.0];
/// Seconds of shaking, as in the game, and of drops landing after.
const SHAKING: f32 = 0.9;
const LENGTH: f32 = 1.2;
/// Seconds between the small rubs that make up the rustle.
const RUB: f32 = 0.003;

impl Samples {
    /// Feathers shaken dry: a rustle beating with the swings of the body,
    /// a soft flap at each swing, and the spray landing around.
    pub(crate) fn shake(seed: u32) -> Self {
        let mut dice = Dice::new(seed.wrapping_add(420));
        let rate = dice.between(8.0, 10.0);
        let mut rustle = Bands::new(seed.wrapping_add(421), &RUSTLE, |hz| 60.0 + hz * 0.01);
        let mut rings = Rings::default();
        let mut drops: Vec<f32> = (0..dice.between(25.0, 45.0) as usize)
            .map(|_| 0.1 + (dice.unit() + dice.unit()) / 2.0 * (LENGTH - 0.15))
            .collect();
        drops.sort_by(|a, b| b.total_cmp(a));
        let mut next_rub = 0.0;
        let mut last_swing = 0.0;
        Self::generate(LENGTH, |t| {
            let envelope = (PI * (t / SHAKING).min(1.0)).sin();
            let swing = (TAU * rate * t).sin().abs();
            while next_rub <= t {
                let strength = swing.powi(3) * envelope * dice.between(0.3, 1.0);
                rustle.hit(|hz| 0.4 * strength * (2_000.0 / hz).sqrt());
                next_rub += RUB;
            }
            // A soft flap as each swing turns.
            if swing < last_swing && last_swing > 0.98 && envelope > 0.1 {
                rings.add(Ring::mode(dice.spread(150.0, 260.0), 0.6, 0.03 * envelope));
            }
            last_swing = swing;
            while drops.last().is_some_and(|&at| at <= t) {
                drops.pop();
                rings.add(Ring::impact(
                    dice.spread(1_500.0, 5_000.0),
                    dice.between(0.05, 0.2),
                ));
            }
            rustle.next() + rings.next()
        })
        .normalized(0.7)
    }
}
