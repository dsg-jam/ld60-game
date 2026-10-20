use std::f32::consts::{PI, TAU};

use super::voice::{BLOCK, RATE};

/// Deterministic white noise from -1 to 1.
#[derive(Debug)]
pub(super) struct Noise(u32);

/// A state variable filter after Simper, "Linear Trapezoidal Optimised SVF",
/// which stays smooth while its cutoff moves.
#[derive(Debug, Default)]
pub(super) struct Svf {
    a1: f32,
    a2: f32,
    a3: f32,
    ic1: f32,
    ic2: f32,
}

/// The low and band pass outputs of an [`Svf`].
#[derive(Clone, Copy, Debug)]
pub(super) struct Bands {
    pub(super) low: f32,
    pub(super) band: f32,
}

/// A sine and cosine pair that turns a little each sample, cheaper than
/// calling `sin`.
#[derive(Debug)]
pub(super) struct Tone {
    x: f32,
    y: f32,
    cos: f32,
    sin: f32,
}

/// Noise that wanders slowly around 0 with a spread of about 1. It moves
/// once per block.
#[derive(Debug)]
pub(super) struct Wander {
    noise: Noise,
    pull: f32,
    first: f32,
    value: f32,
    scale: f32,
}

/// A value that follows its target with a time constant.
#[derive(Debug)]
pub(super) struct Glide {
    value: f32,
    rate: f32,
}

impl Noise {
    pub(super) fn new(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9) | 1)
    }

    pub(super) fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    /// From 0 to 1.
    pub(super) fn unit(&mut self) -> f32 {
        (self.next() + 1.0) / 2.0
    }
}

impl Svf {
    /// Moves the cutoff or center to `hz` with quality `q`.
    pub(super) fn tune(&mut self, hz: f32, q: f32) {
        let g = (PI * hz.clamp(1.0, RATE as f32 * 0.45) / RATE as f32).tan();
        self.a1 = 1.0 / (1.0 + g * (g + 1.0 / q));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    pub(super) fn next(&mut self, x: f32) -> Bands {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        Bands { low: v2, band: v1 }
    }
}

impl Tone {
    pub(super) fn new(phase: f32) -> Self {
        Self {
            x: phase.cos(),
            y: phase.sin(),
            cos: 1.0,
            sin: 0.0,
        }
    }

    /// Sets the frequency and pulls the amplitude back to 1. Call once per
    /// block.
    pub(super) fn tune(&mut self, hz: f32) {
        let step = TAU * hz / RATE as f32;
        self.cos = step.cos();
        self.sin = step.sin();
        let fix = 1.5 - 0.5 * (self.x * self.x + self.y * self.y);
        self.x *= fix;
        self.y *= fix;
    }

    /// The cosine of the phase, before it turns on.
    pub(super) fn next(&mut self) -> f32 {
        let x = self.x;
        self.x = x * self.cos - self.y * self.sin;
        self.y = x * self.sin + self.y * self.cos;
        x
    }
}

impl Wander {
    /// Wanders through two one pole low-passes at `hz`.
    pub(super) fn new(hz: f32, seed: u32) -> Self {
        let pull = 1.0 - (-TAU * hz * BLOCK as f32 / RATE as f32).exp();
        let keep = 1.0 - pull;
        // White noise from -1 to 1 has a variance of a third.
        let variance = pull.powi(4) * (1.0 + keep * keep) / (1.0 - keep * keep).powi(3) / 3.0;
        Self {
            noise: Noise::new(seed),
            pull,
            first: 0.0,
            value: 0.0,
            scale: variance.sqrt().recip(),
        }
    }

    pub(super) fn next(&mut self) -> f32 {
        self.first += (self.noise.next() - self.first) * self.pull;
        self.value += (self.first - self.value) * self.pull;
        self.value * self.scale
    }
}

impl Glide {
    /// Starts at `value` and covers about two thirds of the way to a new
    /// target in `seconds`, updated once per block.
    pub(super) fn new(value: f32, seconds: f32) -> Self {
        Self {
            value,
            rate: 1.0 - (-(BLOCK as f32) / (seconds * RATE as f32)).exp(),
        }
    }

    /// Like [`Self::new`], but starts at the first target it follows.
    pub(super) fn from_first(seconds: f32) -> Self {
        Self::new(f32::NAN, seconds)
    }

    pub(super) fn follow(&mut self, target: f32) -> f32 {
        if self.value.is_nan() {
            self.value = target;
        }
        self.value += (target - self.value) * self.rate;
        self.value
    }
}

/// Scales white noise through an [`Svf`] band pass to the loudness it had
/// before, whatever the band. The band pass peaks at `q` and lets through
/// `PI / 2` times `hz / q`.
pub(super) fn band_gain(hz: f32, q: f32) -> f32 {
    (RATE as f32 / (PI * hz * q)).sqrt()
}

/// Share of the way a one pole filter moves each sample at cutoff `hz`.
pub(super) fn one_pole(hz: f32) -> f32 {
    1.0 - (-TAU * hz / RATE as f32).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_keeps_its_pitch_and_level() {
        let mut tone = Tone::new(0.0);
        let mut crossings = 0;
        let mut last = 1.0;
        let mut peak = 0.0_f32;
        for i in 0..RATE as usize {
            if i % BLOCK == 0 {
                tone.tune(440.0);
            }
            let x = tone.next();
            if last < 0.0 && x >= 0.0 {
                crossings += 1;
            }
            last = x;
            peak = peak.max(x.abs());
        }
        assert!((439..=441).contains(&crossings));
        assert!((peak - 1.0).abs() < 0.01);
    }

    #[test]
    fn wander_spreads_about_one() {
        let mut wander = Wander::new(0.5, 3);
        let count = 200_000;
        let power = (0..count).map(|_| wander.next().powi(2)).sum::<f32>() / count as f32;
        assert!((0.6..1.6).contains(&power), "{power}");
    }

    #[test]
    fn band_pass_noise_has_the_level_asked_for() {
        let mut noise = Noise::new(1);
        let mut svf = Svf::default();
        let (hz, q) = (1_000.0, 2.0);
        svf.tune(hz, q);
        let gain = band_gain(hz, q);
        let count = RATE as usize * 4;
        // White noise from -1 to 1 has a power of a third.
        let power = (0..count)
            .map(|_| (svf.next(noise.next()).band * gain).powi(2))
            .sum::<f32>()
            / count as f32
            * 3.0;
        assert!((0.8..1.25).contains(&power), "{power}");
    }
}
