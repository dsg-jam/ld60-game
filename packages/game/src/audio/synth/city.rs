use std::f32::consts::TAU;

use super::{Lowpass, Mode, Noise, Resonator, SAMPLE_RATE, Samples, Vary};

/// Loops fade from their end into their start over this many seconds.
const SEAM: f32 = 0.5;
/// Rain drops hitting hard surfaces ring at these.
const PATTER: [Mode; 4] = [
    Mode::new(1_300.0, 900.0, 1.0),
    Mode::new(2_200.0, 1_200.0, 1.0),
    Mode::new(3_500.0, 1_600.0, 1.0),
    Mode::new(5_400.0, 2_200.0, 1.0),
];

impl Samples {
    /// Steady rain on the street: a soft wash of many far drops, close
    /// drops ticking on hard surfaces and now and then one plinking into a
    /// puddle. Loops.
    pub(in crate::audio) fn rain(seed: u32) -> Self {
        const SECONDS: f32 = 7.0;
        let mut noise = Noise::new(seed.wrapping_add(70));
        let mut dice = Noise::new(seed.wrapping_add(71));
        let mut wash = Lowpass::new(0.3);
        let mut wash_low = Lowpass::new(0.015);
        let mut hiss = Lowpass::new(0.6);
        let mut patter = Resonator::even(&PATTER, 1.0);
        let mut plinks = Self::plinks(seed, SECONDS + SEAM);
        let mut plink = Plink::default();
        Self::generate(SECONDS + SEAM, |t| {
            let white = noise.next();
            let band = wash.next(white);
            let far = band - wash_low.next(band);
            let near = hiss.next(white) - band;
            let mut drops = 0.0;
            for mode in &mut patter {
                // Most drops are small and a few are big.
                let hit = if (dice.next() + 1.0) / 2.0 < 0.0015 {
                    dice.next().powi(3) * 4.0
                } else {
                    0.0
                };
                drops += mode.next(hit);
            }
            if plinks.last().is_some_and(|next| next.at <= t) {
                plink = plinks.pop().unwrap_or_default();
            }
            // Gusts come and go twice over the loop.
            let gust = 0.85 + 0.15 * (TAU * 2.0 * t / SECONDS).sin();
            (far * 0.6 + near * 0.08) * gust + drops * 0.4 + plink.sample(t) * 0.6
        })
        .looped(SEAM)
        .normalized(0.7)
    }

    /// Drops falling into a puddle, as Minnaert bubbles: short tones that
    /// rise as the bubble closes. Sorted to pop the earliest first.
    fn plinks(seed: u32, seconds: f32) -> Vec<Plink> {
        let mut dice = Noise::new(seed.wrapping_add(72));
        let mut roll = |low: f32, high: f32| low + (high - low) * (dice.next() + 1.0) / 2.0;
        let mut plinks = Vec::new();
        let mut at = roll(0.0, 0.6);
        while at < seconds - 0.1 {
            plinks.push(Plink {
                at,
                pitch: roll(700.0, 2_400.0),
                level: roll(0.2, 1.0),
            });
            at += roll(0.08, 0.7);
        }
        plinks.reverse();
        plinks
    }

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
        .looped(SEAM)
        .normalized(0.7)
    }

    /// Plays the end of the samples over their start, `seam` seconds long,
    /// so they loop without a click.
    fn looped(mut self, seam: f32) -> Self {
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

/// One drop falling into a puddle.
#[derive(Clone, Copy, Debug, Default)]
struct Plink {
    at: f32,
    pitch: f32,
    level: f32,
}

impl Plink {
    fn sample(&self, t: f32) -> f32 {
        let t = t - self.at;
        if !(0.0..0.06).contains(&t) {
            return 0.0;
        }
        // Rises by about a third over its short life.
        let phase = TAU * self.pitch * (t + 6.0 * t * t);
        phase.sin() * self.level * (-t * 70.0).exp() * (t * 3_000.0).min(1.0)
    }
}
