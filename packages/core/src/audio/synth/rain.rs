use std::f32::consts::PI;

use super::Samples;
use super::city::SEAM;
use super::drops::{
    Bands, Dice, Hiss, Rainfall, Ring, Rings, Swell, speed_after, strength, terminal_speed,
};

/// How loud the rain loops are, as RMS.
const LEVEL: f32 = 0.05;
/// Drops smaller than this barely make a sound.
const RAINFALL: (f32, f32) = (0.5, 5.0);
/// Octaves that splashes and sheets ring in.
const OCTAVES: [f32; 7] = [125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0];
/// Drops hitting the water of a puddle hit softer than on stone.
const WATER: f32 = 0.4;
/// Drops 0.8 to 1.1 millimeters across always trap a bubble about 0.2
/// millimeters across. Drops from 2.2 millimeters on trap a bigger one
/// about every other time. After Medwin et al., "The anatomy of
/// underwater rain noise" (1992).
const SMALL_BUBBLES: (f32, f32) = (0.8, 1.1);
const BIG_BUBBLES: f32 = 2.2;
/// How much bubbles rise in pitch, after van den Doel.
const RISE: f32 = 0.1;
/// Loss factor of the rusty, bolted steel of the shop's roof.
const SHEET_LOSS: f32 = 0.04;
/// Metal rings longer at its own modes.
const PING_LOSS: f32 = 0.01;
/// Meters the drips fall from the awnings.
const DRIP_HEIGHT: f32 = 3.4;
/// Size of the drops that drip off edges, in millimeters.
const DRIP_SIZE: (f32, f32) = (4.0, 5.5);

impl Samples {
    /// Rain on the asphalt of the road: dense, fine clicks of drops hitting
    /// rough, wet stone, with the spray they throw up. Loops.
    pub(in crate::audio) fn asphalt_rain(seed: u32) -> Self {
        Self::street_rain(seed, 8.3 + 1.7 * seed as f32, (500.0, 8_000.0), 0.5)
    }

    /// Rain on the smoother concrete of the sidewalks, a little brighter
    /// than on asphalt. Loops.
    pub(in crate::audio) fn concrete_rain(seed: u32) -> Self {
        Self::street_rain(seed + 10, 7.1 + 2.3 * seed as f32, (700.0, 11_000.0), 0.35)
    }

    /// Drops landing all over a patch of hard ground, most of them far
    /// away, as clicks pitched from `pitch` and `spray` of a splash each.
    fn street_rain(seed: u32, seconds: f32, pitch: (f32, f32), spray: f32) -> Self {
        const RATE: f32 = 1_800.0;
        let mut dice = Dice::new(seed.wrapping_add(70));
        let mut swell = Swell::new(&mut dice, seconds, 1.5, 0.25);
        let rainfall = Rainfall::new(RAINFALL.0, RAINFALL.1);
        let mut impacts = Rings::default();
        let mut splash = Bands::new(seed.wrapping_add(71), &OCTAVES[4..], |_| 300.0);
        let mut far = Hiss::new(seed.wrapping_add(72), 400.0);
        let mut next = 0.0;
        Self::generate(seconds + SEAM, |t| {
            let gust = swell.at(t);
            while next <= t {
                let size = rainfall.size(&mut dice);
                // Spread over a disc, so further drops are more common.
                let distance = dice.between(4.0, 25.0).sqrt();
                let hit = strength(size, terminal_speed(size)) / distance;
                let tone = (2.0 / size).powf(0.3);
                impacts.add(Ring::impact(dice.spread(pitch.0, pitch.1) * tone, hit));
                splash.hit(|hz| hit * spray * pink(hz));
                next += dice.gap(RATE * gust);
            }
            impacts.next() + splash.next() + far.next() * 0.05 * gust
        })
        .looped(SEAM)
        .leveled(LEVEL)
    }

    /// Rain on a puddle: soft slaps of drops on water, the high, short
    /// ring of the tiny bubbles that drops of one size always trap, and
    /// now and then the lower plink of a bubble under a big drop. Loops.
    pub(in crate::audio) fn puddle_rain(seed: u32) -> Self {
        const RATE: f32 = 1_500.0;
        let seconds = 7.3 + 1.6 * seed as f32;
        let mut dice = Dice::new(seed.wrapping_add(80));
        let mut swell = Swell::new(&mut dice, seconds, 1.3, 0.25);
        let rainfall = Rainfall::new(RAINFALL.0, RAINFALL.1);
        let mut rings = Rings::default();
        let mut splash = Bands::new(seed.wrapping_add(81), &OCTAVES[4..], |_| 150.0);
        let mut next = 0.0;
        Self::generate(seconds + SEAM, |t| {
            let gust = swell.at(t);
            while next <= t {
                let size = rainfall.size(&mut dice);
                let distance = dice.between(1.0, 4.0).sqrt();
                let hit = strength(size, terminal_speed(size)) / distance;
                let tone = (2.0 / size).powf(0.3);
                rings.add(Ring::impact(
                    dice.spread(600.0, 6_000.0) * tone,
                    hit * WATER,
                ));
                splash.hit(|hz| hit * 0.3 * pink(hz));
                if (SMALL_BUBBLES.0..SMALL_BUBBLES.1).contains(&size) {
                    let radius = dice.between(0.000_16, 0.000_3);
                    rings.add(Ring::bubble(radius, RISE, hit * 1.5));
                } else if size > BIG_BUBBLES && dice.chance(0.5) {
                    // Bigger drops trap bigger bubbles, which ring lower.
                    let pitch = 10_000.0 * (BIG_BUBBLES / size).powf(1.5) * dice.between(0.6, 1.2);
                    rings.add(Ring::bubble(3.0 / pitch, RISE, hit * 0.6));
                }
                next += dice.gap(RATE * gust);
            }
            rings.next() + splash.next()
        })
        .looped(SEAM)
        .leveled(LEVEL)
    }

    /// Rain on a taut canvas awning: each drop presses the cloth in, a
    /// soft, dull pat in its many low modes, and big drops thump at a
    /// pitch of their own. Loops.
    pub(in crate::audio) fn awning_rain(seed: u32) -> Self {
        const RATE: f32 = 1_200.0;
        let seconds = 6.7 + 1.9 * seed as f32;
        let mut dice = Dice::new(seed.wrapping_add(90));
        let mut swell = Swell::new(&mut dice, seconds, 1.4, 0.25);
        let rainfall = Rainfall::new(RAINFALL.0, RAINFALL.1);
        let mut rings = Rings::default();
        // Cloth is heavily damped, the more so the higher it rings.
        let mut cloth = Bands::new(seed.wrapping_add(91), &OCTAVES[1..6], |hz| 80.0 + hz * 0.3);
        let mut next = 0.0;
        Self::generate(seconds + SEAM, |t| {
            let gust = swell.at(t);
            while next <= t {
                let size = rainfall.size(&mut dice);
                let distance = dice.between(1.0, 2.5).sqrt();
                let hit = strength(size, terminal_speed(size)) / distance;
                cloth.hit(|hz| hit * (hz / 300.0).min(1.0).powi(2) / (1.0 + hz / 700.0));
                rings.add(Ring::impact(dice.spread(1_500.0, 5_000.0), hit * 0.08));
                if size > 2.0 {
                    rings.add(Ring::mode(dice.spread(150.0, 400.0), 0.25, hit * 0.3));
                }
                next += dice.gap(RATE * gust);
            }
            rings.next() + cloth.next()
        })
        .looped(SEAM)
        .leveled(LEVEL)
    }

    /// Rain drumming on the corrugated steel roof right above the player.
    /// The sheet has too many modes to tell apart, so it rings like noise
    /// that dies down as fast as its rust and bolts damp it. Big drops make
    /// it ping at a few of its modes. Loops.
    pub(in crate::audio) fn roof_rain(seed: u32) -> Self {
        const RATE: f32 = 1_800.0;
        const MODES: usize = 24;
        let seconds = 10.9 + 1.8 * seed as f32;
        let mut dice = Dice::new(seed.wrapping_add(110));
        let mut swell = Swell::new(&mut dice, seconds, 1.2, 0.3);
        let rainfall = Rainfall::new(RAINFALL.0, RAINFALL.1);
        let modes: Vec<f32> = (0..MODES).map(|_| dice.spread(500.0, 5_000.0)).collect();
        let mut rings = Rings::default();
        let mut sheet = Bands::new(seed.wrapping_add(111), &OCTAVES[1..], |hz| {
            PI * hz * SHEET_LOSS
        });
        let mut next = 0.0;
        Self::generate(seconds + SEAM, |t| {
            let gust = swell.at(t);
            while next <= t {
                let size = rainfall.size(&mut dice);
                let distance = dice.between(1.0, 2.0).sqrt();
                let hit = strength(size, terminal_speed(size)) / distance;
                sheet.hit(|hz| hit * 0.1 * (hz / 1_000.0).min(1.0).sqrt() / (1.0 + hz / 6_000.0));
                rings.add(Ring::impact(dice.spread(2_000.0, 12_000.0), hit * 0.15));
                if size > 2.5 && dice.chance(0.5) {
                    let mode = modes[(dice.unit() * MODES as f32) as usize % MODES];
                    rings.add(Ring::mode(mode, PING_LOSS, hit * 0.02));
                }
                next += dice.gap(RATE * gust);
            }
            rings.next() + sheet.next()
        })
        .looped(SEAM)
        .leveled(LEVEL)
    }

    /// A big drop off an awning hitting the sidewalk: a hard knock with a
    /// little body, then the droplets it throws up landing around it.
    pub(in crate::audio) fn drip(seed: u32) -> Self {
        let mut dice = Dice::new(seed.wrapping_add(130));
        let size = dice.between(DRIP_SIZE.0, DRIP_SIZE.1);
        let hit = strength(size, speed_after(size, DRIP_HEIGHT));
        let mut rings = Rings::default();
        rings.add(Ring::impact(dice.spread(800.0, 2_500.0), hit));
        rings.add(Ring::mode(dice.spread(180.0, 400.0), 0.3, hit * 0.05));
        let mut spray = Bands::new(seed.wrapping_add(131), &OCTAVES[3..], |_| 120.0);
        spray.hit(|hz| hit * 0.15 * pink(hz));
        let mut droplets: Vec<(f32, Ring)> = (0..dice.between(4.0, 10.0) as usize)
            .map(|_| {
                let at = dice.between(0.004, 0.06);
                let ring = Ring::impact(
                    dice.spread(2_500.0, 10_000.0),
                    hit * dice.between(0.03, 0.15),
                );
                (at, ring)
            })
            .collect();
        Self::generate(0.25, |t| {
            droplets.retain(|&(at, ring)| {
                if at <= t {
                    rings.add(ring);
                }
                at > t
            });
            rings.next() + spray.next()
        })
        .normalized(0.8)
    }

    /// A big drop off an awning falling into a puddle: a soft slap of
    /// water and its splash, mostly the high plink of the bubble it traps,
    /// and the splash falling back in droplets.
    pub(in crate::audio) fn plunk(seed: u32) -> Self {
        let mut dice = Dice::new(seed.wrapping_add(140));
        let size = dice.between(DRIP_SIZE.0, DRIP_SIZE.1);
        let hit = strength(size, speed_after(size, DRIP_HEIGHT));
        let mut rings = Rings::default();
        rings.add(Ring::impact(dice.spread(600.0, 2_500.0), hit * WATER));
        let mut splash = Bands::new(seed.wrapping_add(141), &OCTAVES[2..], |_| 60.0);
        splash.hit(|hz| hit * 0.1 * pink(hz));
        let mut later = Vec::new();
        // Not every drop traps a bubble.
        if seed % 6 != 5 {
            let pitch = dice.spread(2_500.0, 8_000.0);
            later.push((
                dice.between(0.005, 0.02),
                Ring::bubble(3.0 / pitch, RISE, hit * 0.8),
            ));
        }
        if dice.chance(0.5) {
            let pitch = dice.spread(5_000.0, 12_000.0);
            later.push((
                dice.between(0.1, 0.3),
                Ring::bubble(3.0 / pitch, RISE, hit * 0.25),
            ));
        }
        for _ in 0..dice.between(3.0, 7.0) as usize {
            let ring = Ring::impact(
                dice.spread(1_500.0, 6_000.0),
                hit * WATER * dice.between(0.05, 0.2),
            );
            later.push((dice.between(0.05, 0.25), ring));
        }
        Self::generate(0.45, |t| {
            later.retain(|&(at, ring)| {
                if at <= t {
                    rings.add(ring);
                }
                at > t
            });
            rings.next() + splash.next()
        })
        .normalized(0.8)
    }

    /// Scales the samples to `rms`, but never past a peak of 1.
    fn leveled(mut self, rms: f32) -> Self {
        let sum = self.0.iter().map(|sample| sample * sample).sum::<f32>();
        let peak = self
            .0
            .iter()
            .fold(0.0_f32, |max, sample| max.max(sample.abs()));
        if sum > 0.0 {
            let gain = (rms / (sum / self.0.len() as f32).sqrt()).min(0.95 / peak);
            for sample in &mut self.0 {
                *sample *= gain;
            }
        }
        self
    }
}

/// How hard to hit a band of noise at `hz`, so that each octave gets the
/// same energy.
fn pink(hz: f32) -> f32 {
    (1_000.0 / hz).sqrt()
}
