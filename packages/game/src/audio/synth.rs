mod city;
mod drops;
mod keys;
mod lamp;
mod paper;
mod rain;
mod rings;
mod room;

use std::f32::consts::{PI, TAU};

use self::room::Room;

pub(super) const SAMPLE_RATE: u32 = 44_100;

/// Mono samples from -1 to 1.
#[derive(Clone, Debug)]
pub(super) struct Samples(Vec<f32>);

impl Samples {
    /// Two soft bell notes going up, for a finished job.
    pub(super) fn chime() -> Self {
        const NOTES: [(f32, f32); 2] = [(0.0, 1_046.5), (0.14, 1_318.5)];
        Self::generate(1.4, |t| {
            NOTES
                .iter()
                .filter(|(start, _)| t >= *start)
                .map(|&(start, pitch)| {
                    let t = t - start;
                    let attack = (t * 400.0).min(1.0);
                    let bell = (TAU * pitch * t).sin() + 0.3 * (TAU * pitch * 2.76 * t).sin();
                    bell * attack * (-t * 3.5).exp()
                })
                .sum()
        })
        .normalized(0.7)
    }

    /// A rubber stamp hitting paper on a wooden desk.
    pub(super) fn thunk() -> Self {
        let mut noise = Noise::new(4);
        let mut thud = Lowpass::new(0.05);
        Self::generate(0.25, |t| {
            let attack = (t * 2_000.0).min(1.0);
            let body = (TAU * 95.0 * t).sin() * (-t * 28.0).exp();
            let slap = thud.next(noise.next()) * 6.0 * (-t * 90.0).exp();
            (body + slap) * attack
        })
        .normalized(0.8)
    }

    /// A small brass service bell.
    pub(super) fn ding() -> Self {
        const PITCH: f32 = 2_093.0;
        Self::generate(2.0, |t| {
            let attack = (t * 800.0).min(1.0);
            let ring = (TAU * PITCH * t).sin()
                + 0.4 * (TAU * PITCH * 2.76 * t).sin() * (-t * 2.0).exp()
                + 0.2 * (TAU * PITCH * 5.4 * t).sin() * (-t * 5.0).exp();
            ring * attack * (-t * 1.8).exp()
        })
        .normalized(0.6)
    }

    /// A knob snapping into its next detent: the spring catching makes a
    /// sharp tick in the small plastic and metal parts, then the knob
    /// settles with a softer one. `seed` varies the parts' pitch and timing.
    pub(super) fn detent(seed: u32) -> Self {
        /// Small, stiff parts ring high and stop quickly.
        const SNAP: [Mode; 3] = [
            Mode::new(3_100.0, 2_600.0, 1.0),
            Mode::new(5_300.0, 3_400.0, 0.6),
            Mode::new(8_200.0, 4_500.0, 0.35),
        ];
        /// The knob and the case, lower and a little longer.
        const SETTLE: [Mode; 2] = [
            Mode::new(1_400.0, 1_100.0, 1.0),
            Mode::new(2_350.0, 1_500.0, 0.5),
        ];
        let mut vary = Vary::new(seed);
        Self::strike(
            seed,
            vary.by(0.06),
            &[
                Hit::new(0.0, 0.0003, 1.0, &SNAP),
                Hit::new(0.006 * vary.by(0.3), 0.0006, 0.35 * vary.by(0.3), &SETTLE),
            ],
        )
    }

    /// One soft tick of a volume knob's ratchet, much quieter and duller
    /// than a detent.
    pub(super) fn ratchet(seed: u32) -> Self {
        const TICK: [Mode; 2] = [
            Mode::new(1_900.0, 3_000.0, 1.0),
            Mode::new(3_600.0, 4_000.0, 0.4),
        ];
        let mut vary = Vary::new(seed);
        Self::strike(seed, vary.by(0.1), &[Hit::new(0.0, 0.0005, 1.0, &TICK)])
    }

    /// A push button latching down: the cap bottoms out with a dull knock,
    /// and the latch catches with a bright tick right after.
    pub(super) fn latch(seed: u32) -> Self {
        const KNOCK: [Mode; 2] = [
            Mode::new(900.0, 700.0, 1.0),
            Mode::new(1_700.0, 1_000.0, 0.5),
        ];
        const CATCH: [Mode; 2] = [
            Mode::new(4_200.0, 3_000.0, 1.0),
            Mode::new(6_900.0, 4_000.0, 0.5),
        ];
        let mut vary = Vary::new(seed);
        Self::strike(
            seed,
            vary.by(0.05),
            &[
                Hit::new(0.0, 0.0008, 1.0, &KNOCK),
                Hit::new(0.012 * vary.by(0.2), 0.0003, 0.6, &CATCH),
            ],
        )
    }

    /// A push button springing back up: the latch lets go with a tick, and
    /// the cap hits its stop with a light clack.
    pub(super) fn unlatch(seed: u32) -> Self {
        const RELEASE: [Mode; 2] = [
            Mode::new(3_800.0, 3_200.0, 1.0),
            Mode::new(6_100.0, 4_200.0, 0.4),
        ];
        const STOP: [Mode; 2] = [
            Mode::new(1_300.0, 1_200.0, 1.0),
            Mode::new(2_600.0, 1_700.0, 0.6),
        ];
        let mut vary = Vary::new(seed);
        Self::strike(
            seed,
            vary.by(0.05),
            &[
                Hit::new(0.0, 0.0003, 0.5, &RELEASE),
                Hit::new(0.008 * vary.by(0.2), 0.0005, 1.0, &STOP),
            ],
        )
    }

    /// Short contacts that make small parts ring, after van den Doel, Kry
    /// and Pai, "FoleyAutomatic": each hit drives its own bank of modes.
    /// All modes are tuned by `pitch`.
    fn strike(seed: u32, pitch: f32, hits: &[Hit]) -> Self {
        const SECONDS: f32 = 0.06;
        let mut noise = Noise::new(seed.wrapping_add(21));
        let mut banks: Vec<_> = hits
            .iter()
            .map(|hit| Resonator::bank(hit.modes, pitch))
            .collect();
        Self::generate(SECONDS, |t| {
            // The contact itself is a short burst of noise.
            let scrape = noise.next() * (-t * 6_000.0).exp() * 0.02;
            hits.iter()
                .zip(&mut banks)
                .map(|(hit, bank)| {
                    let force = Self::pulse(t - hit.at, hit.width) * hit.level;
                    bank.iter_mut().map(|mode| mode.next(force)).sum::<f32>()
                })
                .sum::<f32>()
                + scrape
        })
        .normalized(0.8)
    }

    /// A burst of radio static between stations: hiss with crackles, the
    /// whistle of a carrier sweeping past and a moment of another station.
    /// `seed` varies all of it.
    pub(super) fn radio_static(seed: u32) -> Self {
        let mut dice = Noise::new(seed.wrapping_add(30));
        let mut roll = |low: f32, high: f32| low + (high - low) * (dice.next() + 1.0) / 2.0;
        let seconds = roll(0.25, 0.55);
        let crackles = roll(0.001, 0.008);
        let tone = roll(0.15, 0.5);
        let whistle_from = roll(400.0, 2_500.0);
        let whistle_to = roll(400.0, 2_500.0);
        let whistle = roll(0.0, 0.25);
        let bleed_at = roll(0.2, 0.7) * seconds;
        let mut noise = Noise::new(seed.wrapping_add(31));
        let mut crackle_dice = Noise::new(seed.wrapping_add(32));
        let mut hiss = Lowpass::new(tone);
        let mut voice = Lowpass::new(0.05);
        let mut phase = 0.0;
        Self::generate(seconds, |t| {
            let along = t / seconds;
            let soft = hiss.next(noise.next());
            let crackle = if (crackle_dice.next() + 1.0) / 2.0 < crackles {
                noise.next()
            } else {
                0.0
            };
            phase +=
                TAU * (whistle_from + (whistle_to - whistle_from) * along) / SAMPLE_RATE as f32;
            // Muffled sound of a station, there for a moment.
            let near = (-((t - bleed_at) * 30.0).powi(2)).exp();
            let bleed = voice.next(noise.next()) * 4.0 * near;
            let fade = (1.0 - along).sqrt() * (t * 200.0).min(1.0);
            (soft * 0.6 + crackle + phase.sin() * whistle + bleed) * fade
        })
        .normalized(0.8)
    }

    /// A smooth bump `width` seconds long, starting at 0.
    fn pulse(t: f32, width: f32) -> f32 {
        if (0.0..width).contains(&t) {
            (t / width * PI).sin()
        } else {
            0.0
        }
    }

    /// Adds the echoes and the tail of `room`. The tail is velvet noise,
    /// after Jarvelainen and Karjalainen, "Reverberation modeling using
    /// velvet noise", and gets duller as it dies down.
    fn in_room(mut self, seed: u32, room: &Room) -> Self {
        /// Reflections per second in the tail.
        const DENSITY: f32 = 1_500.0;
        /// The tail loses its highs above this many hertz.
        const DULL: f32 = 3_000.0;
        let rate = SAMPLE_RATE as f32;
        let mut dice = Noise::new(seed.wrapping_add(90));
        let start = room.tail_start * rate;
        let spacing = rate / DENSITY;
        let mut tail: Vec<(usize, f32)> = (0..((room.decay - room.tail_start).max(0.0) * DENSITY)
            as usize)
            .map(|index| {
                let delay = start + spacing * (index as f32 + (dice.next() + 1.0) / 2.0);
                let sign = if dice.next() < 0.0 { -1.0 } else { 1.0 };
                (
                    delay as usize,
                    sign * (-6.9 * delay / rate / room.decay).exp(),
                )
            })
            .collect();
        let energy: f32 = tail.iter().map(|(_, gain)| gain * gain).sum();
        let scale = if energy > 0.0 {
            (room.tail / energy).sqrt()
        } else {
            0.0
        };
        for (_, gain) in &mut tail {
            *gain *= scale;
        }
        let echoes: Vec<(usize, f32)> = room
            .echoes
            .iter()
            .map(|echo| ((echo.delay * rate) as usize, echo.gain))
            .collect();
        let length = tail
            .last()
            .map_or(0, |&(delay, _)| delay)
            .max(start as usize);
        // Only the part of the sound that is still audible has to be
        // reflected.
        let peak = self
            .0
            .iter()
            .fold(0.0_f32, |max, sample| max.max(sample.abs()));
        let heard = self
            .0
            .iter()
            .rposition(|sample| sample.abs() > peak * 1e-3)
            .map_or(0, |last| last + 1);
        let dry = self.0.len();
        let mut clear = vec![0.0; dry + length];
        let mut dull = vec![0.0; dry + length];
        for (taps, wet) in [(&echoes, &mut clear), (&tail, &mut dull)] {
            for &(delay, gain) in taps {
                for (out, sample) in wet[delay..].iter_mut().zip(&self.0[..heard]) {
                    *out += sample * gain;
                }
            }
        }
        let mut filter = Lowpass::at(DULL);
        self.0.resize(dry + length, 0.0);
        for ((sample, clear), dull) in self.0.iter_mut().zip(clear).zip(dull) {
            *sample += clear + filter.next(dull);
        }
        self
    }

    fn normalized(mut self, peak: f32) -> Self {
        let max = self
            .0
            .iter()
            .fold(0.0_f32, |max, sample| max.max(sample.abs()));
        if max > 0.0 {
            for sample in &mut self.0 {
                *sample *= peak / max;
            }
        }
        self
    }

    /// 16 bit mono WAV file.
    pub(super) fn wav(&self) -> Vec<u8> {
        let data_len = (self.0.len() * 2) as u32;
        let mut wav = Vec::with_capacity(44 + data_len as usize);
        wav.extend(b"RIFF");
        wav.extend((36 + data_len).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16_u32.to_le_bytes());
        wav.extend(1_u16.to_le_bytes());
        wav.extend(1_u16.to_le_bytes());
        wav.extend(SAMPLE_RATE.to_le_bytes());
        wav.extend((SAMPLE_RATE * 2).to_le_bytes());
        wav.extend(2_u16.to_le_bytes());
        wav.extend(16_u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend(data_len.to_le_bytes());
        for sample in &self.0 {
            wav.extend(((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16).to_le_bytes());
        }
        wav
    }

    fn generate(seconds: f32, mut sample: impl FnMut(f32) -> f32) -> Self {
        let count = (seconds * SAMPLE_RATE as f32) as usize;
        Self(
            (0..count)
                .map(|i| sample(i as f32 / SAMPLE_RATE as f32))
                .collect(),
        )
    }
}

/// Deterministic white noise from -1 to 1.
pub(super) struct Noise(u32);

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
}

/// A way something rings when struck: its frequency in hertz, how fast it
/// dies down per second, and how loud it is.
#[derive(Clone, Copy, Debug)]
struct Mode {
    frequency: f32,
    decay: f32,
    gain: f32,
}

impl Mode {
    const fn new(frequency: f32, decay: f32, gain: f32) -> Self {
        Self {
            frequency,
            decay,
            gain,
        }
    }

    /// `count` modes at random between `low` and `high` hertz, so a part
    /// rings with a cluster of close tones instead of a few clean ones.
    /// Each mode rings for about `q` cycles and is as loud as `gain` or a
    /// little quieter.
    fn scatter(vary: &mut Vary, low: f32, high: f32, count: usize, q: f32, gain: f32) -> Vec<Self> {
        let step = (high / low).ln() / count as f32;
        (0..count)
            .map(|index| {
                let along = index as f32 + vary.by(0.5) - 0.5;
                let frequency = low * (step * along).exp();
                Self::new(
                    frequency,
                    PI * frequency / (q * vary.by(0.3)),
                    gain * (1.0 - 0.2 * vary.by(1.0)),
                )
            })
            .collect()
    }
}

/// A contact `width` seconds long at `at` seconds, driving `modes`.
struct Hit<'a> {
    at: f32,
    width: f32,
    level: f32,
    modes: &'a [Mode],
}

impl<'a> Hit<'a> {
    const fn new(at: f32, width: f32, level: f32, modes: &'a [Mode]) -> Self {
        Self {
            at,
            width,
            level,
            modes,
        }
    }
}

/// Small random changes, so takes of a sound differ.
struct Vary(Noise);

impl Vary {
    fn new(seed: u32) -> Self {
        Self(Noise::new(seed.wrapping_add(20)))
    }

    /// A factor from `1 - amount` to `1 + amount`.
    fn by(&mut self, amount: f32) -> f32 {
        1.0 + self.0.next() * amount
    }
}

/// A [`Mode`] as a two pole resonator, which rings when driven.
struct Resonator {
    a1: f32,
    a2: f32,
    gain: f32,
    y1: f32,
    y2: f32,
}

impl Resonator {
    /// Resonators for `modes`, all tuned by `pitch`.
    fn bank(modes: &[Mode], pitch: f32) -> Vec<Self> {
        modes
            .iter()
            .map(|mode| {
                let radius = (-mode.decay / SAMPLE_RATE as f32).exp();
                let angle = TAU * mode.frequency * pitch / SAMPLE_RATE as f32;
                Self {
                    a1: 2.0 * radius * angle.cos(),
                    a2: -radius * radius,
                    gain: mode.gain,
                    y1: 0.0,
                    y2: 0.0,
                }
            })
            .collect()
    }

    /// Like [`Self::bank`], but each mode rings as loud as its gain however
    /// high it is.
    fn even(modes: &[Mode], pitch: f32) -> Vec<Self> {
        let mut bank = Self::bank(modes, pitch);
        for (resonator, mode) in bank.iter_mut().zip(modes) {
            resonator.gain *= (TAU * mode.frequency * pitch / SAMPLE_RATE as f32).sin();
        }
        bank
    }

    fn next(&mut self, input: f32) -> f32 {
        let y = input * self.gain + self.a1 * self.y1 + self.a2 * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// One pole low-pass filter. `amount` from 0 (closed) to 1 (open).
struct Lowpass {
    amount: f32,
    value: f32,
}

impl Lowpass {
    fn new(amount: f32) -> Self {
        Self { amount, value: 0.0 }
    }

    /// Opens up to `hertz`.
    fn at(hertz: f32) -> Self {
        Self::new(1.0 - (-TAU * hertz / SAMPLE_RATE as f32).exp())
    }

    fn next(&mut self, input: f32) -> f32 {
        self.value += (input - self.value) * self.amount;
        self.value
    }
}

/// Lets through what lies between two frequencies, falling off by 12 dB
/// per octave on both sides.
struct Band {
    low: [Lowpass; 2],
    high: [Lowpass; 2],
}

impl Band {
    fn new(low: f32, high: f32) -> Self {
        Self {
            low: [Lowpass::at(low), Lowpass::at(low)],
            high: [Lowpass::at(high), Lowpass::at(high)],
        }
    }

    fn next(&mut self, input: f32) -> f32 {
        let mut output = input;
        for filter in &mut self.low {
            output -= filter.next(output);
        }
        for filter in &mut self.high {
            output = filter.next(output);
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_data() {
        let wav = Samples::chime().wav();
        assert_eq!(&wav[..4], b"RIFF");
        let data_len = u32::from_le_bytes(wav[40..44].try_into().unwrap());
        assert_eq!(wav.len(), 44 + data_len as usize);
    }
}
