use std::f32::consts::{PI, TAU};

use super::{Lowpass, Noise, SAMPLE_RATE};

/// Rain in millimeters per hour that the drop sizes are made for, moderate
/// to heavy.
const RAIN_RATE: f32 = 8.0;
const GRAVITY: f32 = 9.81;
/// Rings this much quieter than at their start have died down.
const SILENT: f32 = 1e-3;
/// Rising rings change pitch in steps this many samples apart.
const RISE_EVERY: u32 = 32;

/// Random numbers for timing and sizing drops.
pub(super) struct Dice(Noise);

impl Dice {
    pub(super) fn new(seed: u32) -> Self {
        Self(Noise::new(seed))
    }

    pub(super) fn unit(&mut self) -> f32 {
        ((self.0.next() + 1.0) / 2.0).clamp(1e-6, 1.0)
    }

    pub(super) fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    /// Like [`Self::between`], but as likely to land in any octave.
    pub(super) fn spread(&mut self, low: f32, high: f32) -> f32 {
        low * (high / low).powf(self.unit())
    }

    pub(super) fn chance(&mut self, odds: f32) -> bool {
        self.unit() < odds
    }

    /// Seconds until the next of `rate` events per second that come at
    /// random.
    pub(super) fn gap(&mut self, rate: f32) -> f32 {
        -self.unit().ln() / rate.max(1e-3)
    }
}

/// The drops that land on the ground, from `smallest` to `largest`
/// millimeters across. Sizes spread as in Marshall and Palmer, "The
/// distribution of raindrops with size" (1948), and big drops land more
/// often than that as they fall faster.
#[derive(Clone, Copy, Debug)]
pub(super) struct Rainfall {
    slope: f32,
    smallest: f32,
    largest: f32,
}

impl Rainfall {
    pub(super) fn new(smallest: f32, largest: f32) -> Self {
        Self {
            slope: 4.1 * RAIN_RATE.powf(-0.21),
            smallest,
            largest,
        }
    }

    /// The size of the next drop to land, in millimeters.
    pub(super) fn size(&self, dice: &mut Dice) -> f32 {
        let low = (-self.slope * self.smallest).exp();
        let high = (-self.slope * self.largest).exp();
        loop {
            let size = -(low + (high - low) * dice.unit()).ln() / self.slope;
            if dice.unit() * terminal_speed(self.largest) <= terminal_speed(size) {
                return size;
            }
        }
    }
}

/// A sine that dies down, maybe rising in pitch, turned sample by sample
/// as a phasor.
#[derive(Clone, Copy, Debug)]
pub(super) struct Ring {
    re: f32,
    im: f32,
    turn: (f32, f32),
    step: f32,
    /// How much the step grows every [`RISE_EVERY`] samples.
    rise: f32,
    fade: f32,
    age: u32,
    /// The squared size of the phasor where it has died down.
    end: f32,
}

impl Ring {
    /// A ring at `frequency` that dies down by `decay` per second.
    pub(super) fn new(frequency: f32, decay: f32, level: f32) -> Self {
        let step = TAU * frequency.min(0.45 * SAMPLE_RATE as f32) / SAMPLE_RATE as f32;
        let fade = (-decay / SAMPLE_RATE as f32).exp();
        Self {
            re: level,
            im: 0.0,
            turn: (fade * step.cos(), fade * step.sin()),
            step,
            rise: 0.0,
            fade,
            age: 0,
            end: (level * SILENT).powi(2),
        }
    }

    /// A drop hitting a surface, as in Liu, Cheng and Tong, "Physically
    /// based statistical simulation of rain sound" (2019): it is over
    /// within two periods. `strength` sets its energy, at any pitch.
    pub(super) fn impact(frequency: f32, strength: f32) -> Self {
        Self::new(
            frequency,
            2.0 * frequency,
            strength * (frequency / 1_000.0).sqrt(),
        )
    }

    /// An air bubble `radius` meters across caught under water, as in van
    /// den Doel, "Physically based models for liquid sounds" (2005). It
    /// rings at its Minnaert frequency and rises in pitch by `rise` of
    /// its decay.
    pub(super) fn bubble(radius: f32, rise: f32, level: f32) -> Self {
        let frequency = 3.0 / radius;
        let decay = 0.13 / radius + 0.0072 * radius.powf(-1.5);
        let mut ring = Self::new(frequency, decay, level);
        ring.rise = ring.step * rise * decay * RISE_EVERY as f32 / SAMPLE_RATE as f32;
        ring
    }

    /// A ring at `frequency` that loses `loss` of its energy per period, as
    /// a plate does with that loss factor.
    pub(super) fn mode(frequency: f32, loss: f32, level: f32) -> Self {
        Self::new(frequency, PI * frequency * loss, level)
    }

    fn next(&mut self) -> f32 {
        let (cos, sin) = self.turn;
        let re = self.re * cos - self.im * sin;
        self.im = self.re * sin + self.im * cos;
        self.re = re;
        if self.rise != 0.0 {
            self.age += 1;
            if self.age.is_multiple_of(RISE_EVERY) {
                self.step = (self.step + self.rise).min(0.45 * TAU);
                self.turn = (self.fade * self.step.cos(), self.fade * self.step.sin());
            }
        }
        self.im
    }

    fn sounds(&self) -> bool {
        self.re * self.re + self.im * self.im > self.end
    }
}

/// [`Ring`]s that sound until they die down.
#[derive(Default)]
pub(super) struct Rings(Vec<Ring>);

impl Rings {
    pub(super) fn add(&mut self, ring: Ring) {
        if ring.re != 0.0 {
            self.0.push(ring);
        }
    }

    pub(super) fn next(&mut self) -> f32 {
        let out = self.0.iter_mut().map(Ring::next).sum();
        self.0.retain(Ring::sounds);
        out
    }
}

/// Noise in bands that each die down at their own rate after a hit, for
/// the many close modes of a sheet or the spray of a splash, after Cook,
/// "Physically informed sonic modeling (PhISM)" (1997).
pub(super) struct Bands {
    bands: Vec<Band>,
    noise: Noise,
}

struct Band {
    center: f32,
    filter: Bandpass,
    fade: f32,
    level: f32,
}

impl Bands {
    /// Bands at `centers` hertz, which die down by `decay` of their center
    /// per second, each an octave wide.
    pub(super) fn new(seed: u32, centers: &[f32], decay: impl Fn(f32) -> f32) -> Self {
        Self {
            bands: centers
                .iter()
                .map(|&center| Band {
                    center,
                    filter: Bandpass::new(center, 1.4),
                    fade: (-decay(center) / SAMPLE_RATE as f32).exp(),
                    level: 0.0,
                })
                .collect(),
            noise: Noise::new(seed),
        }
    }

    /// Hits all bands, each as hard as `level` says for its center.
    pub(super) fn hit(&mut self, level: impl Fn(f32) -> f32) {
        for band in &mut self.bands {
            band.level += level(band.center);
        }
    }

    pub(super) fn next(&mut self) -> f32 {
        let white = self.noise.next();
        self.bands
            .iter_mut()
            .map(|band| {
                band.level *= band.fade;
                band.filter.next(white * band.level)
            })
            .sum()
    }
}

/// The hiss of drops too many and too far to tell apart: noise with the
/// same energy in each octave above `low` hertz, after Kellet's pink noise
/// filter.
pub(super) struct Hiss {
    noise: Noise,
    poles: [f32; 3],
    low: Lowpass,
}

impl Hiss {
    pub(super) fn new(seed: u32, low: f32) -> Self {
        Self {
            noise: Noise::new(seed),
            poles: [0.0; 3],
            low: Lowpass::new(1.0 - (-TAU * low / SAMPLE_RATE as f32).exp()),
        }
    }

    pub(super) fn next(&mut self) -> f32 {
        let white = self.noise.next();
        let [a, b, c] = &mut self.poles;
        *a = 0.997_65 * *a + white * 0.099_046;
        *b = 0.963 * *b + white * 0.296_516_4;
        *c = 0.57 * *c + white * 1.052_691_3;
        let pink = (*a + *b + *c + white * 0.184_8) * 0.2;
        pink - self.low.next(pink)
    }
}

/// Band-pass filter with a peak gain of 1, from Bristow-Johnson's "Audio EQ
/// cookbook".
pub(super) struct Bandpass {
    b0: f32,
    a1: f32,
    a2: f32,
    x: [f32; 2],
    y: [f32; 2],
}

impl Bandpass {
    pub(super) fn new(center: f32, q: f32) -> Self {
        let angle = TAU * center.min(0.45 * SAMPLE_RATE as f32) / SAMPLE_RATE as f32;
        let alpha = angle.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self {
            b0: alpha / a0,
            a1: -2.0 * angle.cos() / a0,
            a2: (1.0 - alpha) / a0,
            x: [0.0; 2],
            y: [0.0; 2],
        }
    }

    pub(super) fn next(&mut self, input: f32) -> f32 {
        let y = self.b0 * (input - self.x[1]) - self.a1 * self.y[0] - self.a2 * self.y[1];
        self.x = [input, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

/// The rain getting heavier and lighter: a factor around 1 that glides
/// between random values and comes back to its start after `seconds`.
pub(super) struct Swell {
    knots: Vec<(f32, f32)>,
    seconds: f32,
    /// The knot before the time asked for last.
    at: usize,
}

impl Swell {
    pub(super) fn new(dice: &mut Dice, seconds: f32, every: f32, depth: f32) -> Self {
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
    pub(super) fn at(&mut self, t: f32) -> f32 {
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

/// How fast in meters per second a drop `size` millimeters across falls
/// through still air, after Atlas, Srivastava and Sekhon, "Doppler radar
/// characteristics of precipitation at vertical incidence" (1973).
pub(super) fn terminal_speed(size: f32) -> f32 {
    (9.65 - 10.3 * (-0.6 * size).exp()).max(0.1)
}

/// How fast a drop `size` millimeters across hits the ground after falling
/// `height` meters, with drag.
pub(super) fn speed_after(size: f32, height: f32) -> f32 {
    let terminal = terminal_speed(size);
    terminal * (1.0 - (-2.0 * GRAVITY * height / (terminal * terminal)).exp()).sqrt()
}

/// How hard a drop `size` millimeters across hits at `speed`, so that the
/// sound carries a share of its energy. A drop 2 millimeters across at its
/// terminal speed hits with 1.
pub(super) fn strength(size: f32, speed: f32) -> f32 {
    (size / 2.0).powf(1.5) * speed / terminal_speed(2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_reach_about_their_terminal_speed() {
        assert!((terminal_speed(2.0) - 6.5).abs() < 0.2);
        assert!(speed_after(4.5, 3.4) < terminal_speed(4.5));
        assert!(speed_after(4.5, 3.4) > 6.0);
    }

    #[test]
    fn most_drops_are_small() {
        let rainfall = Rainfall::new(0.5, 5.0);
        let mut dice = Dice::new(1);
        let sizes: Vec<f32> = (0..10_000).map(|_| rainfall.size(&mut dice)).collect();
        let big = sizes.iter().filter(|&&size| size > 2.2).count();
        assert!(sizes.iter().all(|size| (0.5..=5.0).contains(size)));
        assert!(big > 50 && big < 1_000);
    }

    #[test]
    fn rings_die_down() {
        let mut rings = Rings::default();
        rings.add(Ring::impact(2_000.0, 1.0));
        rings.add(Ring::bubble(0.0002, 0.1, 1.0));
        for _ in 0..SAMPLE_RATE / 10 {
            rings.next();
        }
        assert!(rings.0.is_empty());
    }
}
