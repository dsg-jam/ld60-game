//! A local's feet on the wet sidewalk.
//!
//! The striders walk on hard toes with a soft pad behind them. The toe
//! taps the concrete first, then the pad takes the weight with a dull thump
//! that the slab carries low. On a wet sidewalk the pad squeezes the film
//! of water out from under it, and in a puddle the foot slaps the water.

use super::Samples;
use super::drops::{Bands, Dice, Ring, Rings};

/// Octave bands for the hiss of water and the scuff of the toe.
const OCTAVES: [f32; 5] = [500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0];
/// Seconds between the toe and the pad landing.
const TOE_TO_PAD: (f32, f32) = (0.012, 0.03);
/// The pad pushing water out of the way.
const SQUEEZE: f32 = 0.03;
/// The scuff of the toe is darker than white noise by this power of the
/// frequency.
const SCUFF_TILT: f32 = 0.6;

impl Samples {
    /// A foot landing on the sidewalk, wet if `seed` is odd.
    pub(crate) fn footstep(seed: u32) -> Self {
        let mut dice = Dice::new(seed.wrapping_add(400));
        let wet = seed % 2 == 1;
        let mut rings = Rings::default();
        rings.add(Ring::impact(dice.spread(1_400.0, 3_200.0), 0.8));
        let pad_at = dice.between(TOE_TO_PAD.0, TOE_TO_PAD.1);
        let thump = Ring::mode(dice.spread(80.0, 140.0), 0.8, 0.12);
        let knock = Ring::impact(dice.spread(250.0, 600.0), 0.8);
        let mut scuff = Bands::new(seed.wrapping_add(401), &OCTAVES, |hz| 30.0 + hz * 0.02);
        scuff.hit(|hz| 0.02 * (1_000.0 / hz).powf(SCUFF_TILT));
        let mut water = Bands::new(seed.wrapping_add(402), &OCTAVES[1..], |_| 25.0);
        let mut pad = Some((thump, knock));
        Self::generate(0.25, |t| {
            if t >= pad_at
                && let Some((thump, knock)) = pad.take()
            {
                rings.add(thump);
                rings.add(knock);
                if wet {
                    water.hit(|hz| SQUEEZE * 1_000.0 / hz);
                }
            }
            rings.next() + scuff.next() + water.next()
        })
        .normalized(0.8)
    }

    /// A foot coming down in a puddle: the slap of the water, its spray
    /// and the drops falling back.
    pub(crate) fn splash(seed: u32) -> Self {
        let mut dice = Dice::new(seed.wrapping_add(410));
        let mut rings = Rings::default();
        rings.add(Ring::impact(dice.spread(400.0, 1_200.0), 0.6));
        rings.add(Ring::mode(dice.spread(80.0, 140.0), 0.6, 0.5));
        let mut spray = Bands::new(seed.wrapping_add(411), &OCTAVES, |hz| 20.0 + hz * 0.004);
        spray.hit(|hz| 0.12 * (1_000.0 / hz).powf(0.3));
        let mut later: Vec<(f32, Ring)> = (0..dice.between(4.0, 9.0) as usize)
            .map(|_| {
                let ring = Ring::impact(dice.spread(1_500.0, 6_000.0), dice.between(0.03, 0.12));
                (dice.between(0.03, 0.25), ring)
            })
            .collect();
        if dice.chance(0.6) {
            let pitch = dice.spread(1_500.0, 4_000.0);
            later.push((
                dice.between(0.005, 0.03),
                Ring::bubble(3.0 / pitch, 0.1, 0.15),
            ));
        }
        Self::generate(0.4, |t| {
            later.retain(|&(at, ring)| {
                if at <= t {
                    rings.add(ring);
                }
                at > t
            });
            rings.next() + spray.next()
        })
        .normalized(0.8)
    }
}
