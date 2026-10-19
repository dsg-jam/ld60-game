use std::f32::consts::TAU;

use super::city::SEAM;
use super::{Lowpass, Mode, Noise, Resonator, SAMPLE_RATE, Samples};

/// How loud the rain loops are, as RMS.
const LEVEL: f32 = 0.05;
/// The corrugated sheet of the shop's roof. Rust and the ribs damp it, so it
/// rings dull and short.
const SHEET: [Mode; 5] = [
    Mode::new(640.0, 350.0, 0.25),
    Mode::new(1_150.0, 450.0, 1.0),
    Mode::new(1_870.0, 550.0, 0.8),
    Mode::new(2_900.0, 700.0, 0.5),
    Mode::new(4_300.0, 900.0, 0.3),
];
/// Drops hitting the stone and asphalt of the street: short, wide clicks.
const STONE: [Mode; 2] = [
    Mode::new(2_600.0, 3_000.0, 1.0),
    Mode::new(5_200.0, 4_500.0, 0.5),
];
/// Drops thumping on the canvas awnings of the other stalls.
const CANVAS: [Mode; 2] = [Mode::new(380.0, 260.0, 1.0), Mode::new(900.0, 500.0, 0.4)];
/// How much a bubble rises in pitch while it closes, after van den Doel.
const CHIRP: f32 = 0.2;

impl Samples {
    /// Rain on the street, built from single drops as in Farnell, "Designing
    /// Sound": a dull wash of countless far drops, nearer drops clicking on
    /// stone and thumping on awnings, and some ringing in puddles. Loops.
    pub(in crate::audio) fn rain(seed: u32) -> Self {
        let seconds = 9.0 + 2.0 * seed as f32;
        let mut dice = Dice::new(seed.wrapping_add(70));
        let mut swell = Swell::new(&mut dice, seconds, 1.5, 0.3);
        let mut noise = Noise::new(seed.wrapping_add(71));
        let mut wash = Wash::new(0.02, 0.25);
        let mut far = Patter::new(700.0, 0.0004, &STONE, 0.15);
        let mut awnings = Patter::new(70.0, 0.001, &CANVAS, 0.3);
        let mut near = Patter::new(25.0, 0.0003, &STONE, 0.5);
        let mut puddles = Bubbles::new(5.0, 0.6);
        Self::generate(seconds + SEAM, |t| {
            let white = noise.next();
            let gust = swell.at(t);
            wash.next(white) * gust * 0.35
                + far.next(t, gust, white, &mut dice) * 0.6
                + awnings.next(t, gust, white, &mut dice) * 0.12
                + near.next(t, gust, white, &mut dice) * 0.45
                + puddles.next(t, gust, &mut dice) * 0.15
        })
        .looped(SEAM)
        .leveled(LEVEL)
    }

    /// Rain drumming on the roof right above the player, and dripping off
    /// its edge into the puddles in front of the counter. Loops.
    pub(in crate::audio) fn roof_rain(seed: u32) -> Self {
        let seconds = 13.0 + 4.0 * seed as f32;
        let mut dice = Dice::new(seed.wrapping_add(110));
        let mut swell = Swell::new(&mut dice, seconds, 1.2, 0.4);
        let mut noise = Noise::new(seed.wrapping_add(111));
        let mut wash = Wash::new(0.03, 0.4);
        let mut sheet = Patter::new(400.0, 0.0003, &SHEET, 0.5);
        let mut drips = Drips::new(&mut dice, 5);
        let mut drip = Bubbles::new(0.0, 0.7);
        Self::generate(seconds + SEAM, |t| {
            let white = noise.next();
            let gust = swell.at(t);
            drips.fall(t, &mut dice, &mut drip);
            wash.next(white) * gust * 0.4
                + sheet.next(t, gust, white, &mut dice)
                + drip.next(t, 0.0, &mut dice) * 0.15
        })
        .looped(SEAM)
        .leveled(LEVEL)
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

/// Random numbers for timing and sizing drops.
struct Dice(Noise);

impl Dice {
    fn new(seed: u32) -> Self {
        Self(Noise::new(seed))
    }

    fn unit(&mut self) -> f32 {
        ((self.0.next() + 1.0) / 2.0).clamp(1e-6, 1.0)
    }

    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    /// Seconds until the next of `rate` drops per second that fall at random.
    fn gap(&mut self, rate: f32) -> f32 {
        -self.unit().ln() / rate.max(1e-3)
    }

    /// How loud a drop is. Drop sizes spread exponentially, after Marshall
    /// and Palmer, and bigger drops are much louder, so most are faint and a
    /// few stand out.
    fn size(&mut self) -> f32 {
        let size = -self.unit().ln();
        (size * size.sqrt() / 1.3).min(3.0)
    }
}

/// The rain getting heavier and lighter: a factor around 1 that glides
/// between random values and comes back to its start after `seconds`.
struct Swell {
    knots: Vec<(f32, f32)>,
    seconds: f32,
    /// The knot before the time asked for last.
    at: usize,
}

impl Swell {
    fn new(dice: &mut Dice, seconds: f32, every: f32, depth: f32) -> Self {
        let mut knots = Vec::new();
        let mut at = 0.0;
        while at < seconds - every / 2.0 {
            knots.push((at, 1.0 + depth * (2.0 * dice.unit() - 1.0)));
            at += every * dice.between(0.5, 1.5);
        }
        knots.push((seconds, knots[0].1));
        Self {
            knots,
            seconds,
            at: 0,
        }
    }

    /// The factor at `t` seconds, asked for in order.
    fn at(&mut self, t: f32) -> f32 {
        let t = t % self.seconds;
        if t < self.knots[self.at].0 {
            self.at = 0;
        }
        while self.knots[self.at + 1].0 <= t {
            self.at += 1;
        }
        let (from, low) = self.knots[self.at];
        let (to, high) = self.knots[self.at + 1];
        let along = (t - from) / (to - from);
        low + (high - low) * along * along * (3.0 - 2.0 * along)
    }
}

/// The soft hiss of drops too many and too far to tell apart: noise without
/// what is below `low`, dulled by `dull`.
struct Wash {
    low: Lowpass,
    dull: [Lowpass; 2],
}

impl Wash {
    fn new(low: f32, dull: f32) -> Self {
        Self {
            low: Lowpass::new(low),
            dull: [Lowpass::new(dull), Lowpass::new(dull)],
        }
    }

    fn next(&mut self, white: f32) -> f32 {
        let band = white - self.low.next(white);
        let [first, second] = &mut self.dull;
        second.next(first.next(band))
    }
}

/// Drops falling on one surface at `rate` per second. Each is a short tap
/// of noise `contact` seconds long that rings the surface's modes, dulled
/// by `dull` as it travels to the player.
struct Patter {
    rate: f32,
    contact: f32,
    next: f32,
    hit: f32,
    level: f32,
    modes: Vec<Resonator>,
    /// How hard the drop rings each mode, as it hits a different spot.
    spots: Vec<f32>,
    dull: [Lowpass; 2],
}

impl Patter {
    fn new(rate: f32, contact: f32, modes: &[Mode], dull: f32) -> Self {
        Self {
            rate,
            contact,
            next: 0.0,
            hit: f32::NEG_INFINITY,
            level: 0.0,
            modes: Resonator::even(modes, 1.0),
            spots: vec![1.0; modes.len()],
            dull: [Lowpass::new(dull), Lowpass::new(dull)],
        }
    }

    fn next(&mut self, t: f32, gust: f32, white: f32, dice: &mut Dice) -> f32 {
        while self.next <= t {
            self.hit = self.next;
            self.level = dice.size();
            for spot in &mut self.spots {
                *spot = dice.between(0.2, 1.0);
            }
            self.next += dice.gap(self.rate * gust);
        }
        let since = t - self.hit;
        let force = if since < self.contact {
            white * self.level * (1.0 - since / self.contact)
        } else {
            0.0
        };
        let ring = self
            .modes
            .iter_mut()
            .zip(&self.spots)
            .map(|(mode, spot)| mode.next(force * spot))
            .sum();
        let [first, second] = &mut self.dull;
        second.next(first.next(ring))
    }
}

/// Drops falling into water, as Minnaert bubbles after van den Doel,
/// "Physically based models for liquid sounds": each rings at a pitch set
/// by its radius, dies down and rises a little as it closes.
struct Bubbles {
    rate: f32,
    next: f32,
    ringing: Vec<Bubble>,
    dull: Lowpass,
}

#[derive(Clone, Copy, Debug)]
struct Bubble {
    at: f32,
    pitch: f32,
    decay: f32,
    level: f32,
}

impl Bubbles {
    fn new(rate: f32, dull: f32) -> Self {
        Self {
            rate,
            next: 0.0,
            ringing: Vec::new(),
            dull: Lowpass::new(dull),
        }
    }

    /// A bubble of `radius` meters at `at` seconds.
    fn add(&mut self, at: f32, radius: f32, level: f32) {
        let pitch = 3.0 / radius;
        self.ringing.push(Bubble {
            at,
            pitch,
            decay: 0.043 * pitch + 0.0014 * pitch.powf(1.5),
            level,
        });
    }

    /// Adds bubbles at the usual rate, faster in a gust, unless the rate is
    /// 0, and rings all of them.
    fn next(&mut self, t: f32, gust: f32, dice: &mut Dice) -> f32 {
        while self.rate > 0.0 && self.next <= t {
            let at = self.next;
            self.add(at, dice.between(0.0008, 0.003), dice.size().min(2.0));
            self.next += dice.gap(self.rate * gust);
        }
        self.ringing
            .retain(|bubble| (t - bubble.at) * bubble.decay < 7.0);
        let sum = self
            .ringing
            .iter()
            .map(|bubble| bubble.sample(t))
            .sum::<f32>();
        self.dull.next(sum)
    }
}

impl Bubble {
    fn sample(&self, t: f32) -> f32 {
        let t = t - self.at;
        if t < 0.0 {
            return 0.0;
        }
        let phase = TAU * self.pitch * (t + CHIRP * self.decay * t * t / 2.0);
        let attack = (t * SAMPLE_RATE as f32 / 8.0).min(1.0);
        phase.sin() * self.level * attack * (-self.decay * t).exp()
    }
}

/// Spots along the edge of the roof where water gathers and drips off, each
/// at its own steady pace that wavers a little.
struct Drips {
    spots: Vec<(f32, f32)>,
}

impl Drips {
    fn new(dice: &mut Dice, spots: usize) -> Self {
        Self {
            spots: (0..spots)
                .map(|_| {
                    let every = dice.between(0.4, 2.0);
                    (dice.between(0.0, every), every)
                })
                .collect(),
        }
    }

    /// Drops the drips due by `t` into `puddle`. They are big, so they ring
    /// lower than the rain.
    fn fall(&mut self, t: f32, dice: &mut Dice, puddle: &mut Bubbles) {
        for (next, every) in &mut self.spots {
            while *next <= t {
                puddle.add(*next, dice.between(0.004, 0.007), dice.between(0.5, 1.0));
                *next += *every * dice.between(0.7, 1.3);
            }
        }
    }
}
