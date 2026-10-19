use std::f32::consts::TAU;

use super::{Lowpass, Mode, Noise, Resonator, SAMPLE_RATE, Samples, Vary};

/// Loops fade from their end into their start over this many seconds.
pub(super) const SEAM: f32 = 0.5;
/// How much of their highs the sounds of the street keep on their way to
/// the player.
const MUFFLE: f32 = 0.3;
/// Drops of the spray hitting hard surfaces ring at these.
const PATTER: [Mode; 4] = [
    Mode::new(1_300.0, 900.0, 1.0),
    Mode::new(2_200.0, 1_200.0, 1.0),
    Mode::new(3_500.0, 1_600.0, 1.0),
    Mode::new(5_400.0, 2_200.0, 1.0),
];

impl Samples {
    /// Tires rolling on a wet road: the hiss of water squeezed out of the
    /// tread over the low roar of the road, and the faint whine of an
    /// electric motor. Loops.
    pub(in crate::audio) fn tires(seed: u32) -> Self {
        const SECONDS: f32 = 4.0;
        let mut vary = Vary::new(seed.wrapping_add(80));
        // Whole numbers of cycles over the loop, so it joins up.
        let whine = (520.0 * vary.by(0.2) * SECONDS).round() / SECONDS;
        let wobble = (1.0 * vary.by(0.4) * SECONDS).round() / SECONDS;
        let mut noise = Noise::new(seed.wrapping_add(81));
        let mut tread = Lowpass::new(0.45);
        let mut soft = Lowpass::new(0.5);
        let mut tread_low = Lowpass::new(0.09);
        let mut road = Lowpass::new(0.012);
        Self::generate(SECONDS + SEAM, |t| {
            let white = noise.next();
            let band = soft.next(tread.next(white));
            let hiss = (band - tread_low.next(band)) * (1.0 + 0.2 * (TAU * wobble * t).sin());
            let roar = road.next(white) * 6.0;
            let motor = (TAU * whine * t).sin() + 0.4 * (TAU * 2.0 * whine * t).sin();
            hiss + roar + motor * 0.02
        })
        .muffled(MUFFLE)
        .looped(SEAM)
        .normalized(0.7)
    }

    /// A tire hitting a puddle: a wide burst of water thrown up, which
    /// opens bright and settles darker, with the spray falling back in
    /// drops.
    pub(in crate::audio) fn splash(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(90));
        let length = 0.35 * vary.by(0.25);
        let mut noise = Noise::new(seed.wrapping_add(91));
        let mut dice = Noise::new(seed.wrapping_add(92));
        let mut water = Lowpass::new(0.5);
        let mut slosh = Lowpass::new(0.02);
        let mut patter = Resonator::even(&PATTER, vary.by(0.1));
        Self::generate(length + 0.5, |t| {
            let white = noise.next();
            // The burst gets duller as the water falls back.
            water.amount = 0.6 * (-t * 5.0).exp() + 0.05;
            let attack = (t * 60.0).min(1.0);
            let burst = water.next(white) * attack * (-t / length * 3.0).exp();
            let body = slosh.next(white) * 5.0 * attack * (-t * 12.0).exp();
            let falling = (t - 0.05).max(0.0) * (-(t / length) * 2.0).exp() * 4.0;
            let mut drops = 0.0;
            for mode in &mut patter {
                let hit = if (dice.next() + 1.0) / 2.0 < 0.004 * falling {
                    dice.next()
                } else {
                    0.0
                };
                drops += mode.next(hit);
            }
            burst + body + drops * 0.4
        })
        .muffled(MUFFLE)
        .normalized(0.8)
    }

    /// A flying car hovering past: the hum of its lift fans, beating a
    /// little, the rush of the air they push and a high turbine whine.
    /// Loops.
    pub(in crate::audio) fn hover(seed: u32) -> Self {
        const SECONDS: f32 = 4.0;
        let mut vary = Vary::new(seed.wrapping_add(100));
        let cycles = |hz: f32| (hz * SECONDS).round() / SECONDS;
        let hum = cycles(95.0 * vary.by(0.15));
        let beat = hum + 0.5;
        let turbine = cycles(1_900.0 * vary.by(0.15));
        let mut noise = Noise::new(seed.wrapping_add(101));
        let mut rush = Lowpass::new(0.08);
        let mut soft = Lowpass::new(0.15);
        let mut rush_low = Lowpass::new(0.008);
        Self::generate(SECONDS + SEAM, |t| {
            let fan = |hz: f32| {
                (TAU * hz * t).sin()
                    + 0.5 * (TAU * 2.0 * hz * t).sin()
                    + 0.3 * (TAU * 3.0 * hz * t).sin()
            };
            let band = rush.next(soft.next(noise.next()));
            let air = (band - rush_low.next(band)) * 3.0;
            (fan(hum) + fan(beat)) * 0.25 + air + (TAU * turbine * t).sin() * 0.03
        })
        .muffled(MUFFLE)
        .looped(SEAM)
        .normalized(0.7)
    }

    /// Dulls the samples with a two pole low-pass, `amount` from 0 (closed)
    /// to 1 (open).
    pub(super) fn muffled(mut self, amount: f32) -> Self {
        let mut first = Lowpass::new(amount);
        let mut second = Lowpass::new(amount);
        for sample in &mut self.0 {
            *sample = second.next(first.next(*sample));
        }
        self
    }

    /// Plays the end of the samples over their start, `seam` seconds long,
    /// so they loop without a click.
    pub(super) fn looped(mut self, seam: f32) -> Self {
        let fade = (seam * SAMPLE_RATE as f32) as usize;
        let length = self.0.len() - fade;
        for i in 0..fade {
            let mix = i as f32 / fade as f32;
            self.0[i] = self.0[i] * mix.sqrt() + self.0[length + i] * (1.0 - mix).sqrt();
        }
        self.0.truncate(length);
        self
    }
}
