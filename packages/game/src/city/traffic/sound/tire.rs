use std::f32::consts::FRAC_PI_2;

use super::dsp::{Glide, Noise, Svf, Tone, band_gain};
use super::voice::{BLOCK, RATE};

/// Sounds are made for this speed and scale with the speed from there, in
/// meters per second.
const SPEED: f32 = 12.0;
/// A sedan's tire, in meters.
const WIDTH: f32 = 0.205;
/// Tread blocks ring out at this many times the rate they hit the road.
const TREAD: f32 = 1.8;
const ROAR_HZ: f32 = 250.0;
const PUMP_HZ: f32 = 2_000.0;
/// The hiss of a thin film is this high, and falls as the water gets deeper
/// and the drops bigger.
const SPRAY_HZ: f32 = 4_000.0;
const SPRAY_LOWEST_HZ: f32 = 1_500.0;
/// Millimeters of water at which the spray is half as loud as it gets.
const SPRAY_HALF: f32 = 0.8;
/// Air pumped out of the tread falls to half once this much water fills
/// the grooves, in millimeters.
const PUMP_HALF: f32 = 0.3;
/// Water this deep in millimeters damps the tread to half.
const TREAD_HALF: f32 = 20.0;
const SHEET_HZ: f32 = 1_600.0;
const SLOSH_HZ: f32 = 380.0;
/// Millimeters of water above what the tread drains that make the bow wave
/// and splash as loud as at [`SPEED`].
const SPLASH_DEPTH: f32 = 10.0;
/// Millimeters per second of water rising under the tire for the slap of
/// hitting it as loud as its level.
const SLAP_RISE: f32 = 100.0;
/// Seconds for the slap to die down.
const SLAP_DECAY: f32 = 0.08;
/// Seconds that thrown water stays in the air before it falls back.
const FALL: f32 = 0.35;
/// Drops falling back per second, for a splash as loud as at [`SPEED`] and
/// [`SPLASH_DEPTH`].
const DROPS: f32 = 70.0;
/// Radii of the drops that fall back, in meters.
const DROP_RADIUS: (f32, f32) = (0.0008, 0.003);
/// Seconds for the speed and depth to reach a new value.
const GLIDE: f32 = 0.03;

const TREAD_LEVEL: f32 = 1.0;
const ROAR_LEVEL: f32 = 0.6;
const PUMP_LEVEL: f32 = 0.25;
const SPRAY_LEVEL: f32 = 4.0;
const SHEET_LEVEL: f32 = 3.0;
const SLOSH_LEVEL: f32 = 2.2;
const SLAP_LEVEL: f32 = 4.0;
const DROP_LEVEL: f32 = 1.5;

/// A tire on the street.
#[derive(Clone, Copy, Debug)]
pub(super) struct Tire {
    /// In meters.
    pub(super) width: f32,
    /// Weight on the tire, as a share of a sedan's.
    pub(super) load: f32,
    /// Meters between tread blocks.
    pub(super) pitch: f32,
    /// Millimeters of water the grooves of the tread carry away. Deeper
    /// water piles up in front of the tire and splashes.
    pub(super) drain: f32,
}

/// The sound where a tire meets the road, after Sandberg and Ejsmont,
/// "Tyre/Road Noise Reference Book": tread blocks hitting the road, the
/// roar of its texture and air pumped out of the tread. On a wet road water
/// takes the place of the air. A thin film is flung off the tread as a fine
/// hiss. Deeper water the tread can't drain piles up as a bow wave, throws
/// a sheet of water to the sides and falls back as drops.
pub(super) struct TireSound {
    tire: Tire,
    rolling: Noise,
    water: Noise,
    dice: Noise,
    speed: Glide,
    depth: Glide,
    last_depth: Option<f32>,
    tread: Svf,
    roar: Svf,
    pump: Svf,
    spray: Svf,
    sheet: Svf,
    slosh: Svf,
    levels: Levels,
    slap: f32,
    slap_decay: f32,
    /// Water in the air, as drops per second it will rain back down.
    thrown: f32,
    drops: [Drop; 6],
}

#[derive(Default)]
struct Levels {
    tread: f32,
    roar: f32,
    pump: f32,
    spray: f32,
    sheet: f32,
    slosh: f32,
}

/// A drop falling into a puddle, which traps a bubble that rings and rises
/// in pitch as it goes, after van den Doel, "Physically Based Models for
/// Liquid Sounds".
struct Drop {
    tone: Tone,
    hz: f32,
    /// Hertz per second.
    rise: f32,
    level: f32,
    decay: f32,
}

impl Tire {
    pub(super) fn sound(self, seed: u32) -> TireSound {
        TireSound {
            tire: self,
            rolling: Noise::new(seed.wrapping_add(1)),
            water: Noise::new(seed.wrapping_add(2)),
            dice: Noise::new(seed.wrapping_add(3)),
            speed: Glide::from_first(GLIDE),
            depth: Glide::from_first(GLIDE),
            last_depth: None,
            tread: Svf::default(),
            roar: Svf::default(),
            pump: Svf::default(),
            spray: Svf::default(),
            sheet: Svf::default(),
            slosh: Svf::default(),
            levels: Levels::default(),
            slap: 0.0,
            slap_decay: (-(BLOCK as f32) / (SLAP_DECAY * RATE as f32)).exp(),
            thrown: 0.0,
            drops: std::array::from_fn(|_| Drop::silent()),
        }
    }
}

impl TireSound {
    /// Follows the speed in meters per second and the water under the tire
    /// in millimeters.
    pub(super) fn block(&mut self, speed: f32, water: f32) {
        let dt = BLOCK as f32 / RATE as f32;
        let tire = self.tire;
        let speed = self.speed.follow(speed).max(0.0);
        let depth = self.depth.follow(water).max(0.0);
        let fast = speed / SPEED;
        let wide = tire.width / WIDTH;

        let tread_hz = TREAD * speed / tire.pitch;
        self.tread.tune(tread_hz, 1.0);
        let roar_hz = ROAR_HZ * fast.sqrt();
        self.roar.tune(roar_hz, 0.7);
        let pump_hz = PUMP_HZ * fast.powf(0.3);
        self.pump.tune(pump_hz, 1.5);
        let spray_hz = (SPRAY_HZ / (1.0 + depth / 1.5)).max(SPRAY_LOWEST_HZ);
        self.spray.tune(spray_hz, 0.8);

        let levels = &mut self.levels;
        levels.tread = TREAD_LEVEL * fast.powf(1.6) * tire.load.powf(0.3) * wide.sqrt()
            / (1.0 + depth / TREAD_HALF)
            * band_gain(tread_hz, 1.0);
        levels.roar = ROAR_LEVEL * fast.powf(1.3) * tire.load.sqrt() * band_gain(roar_hz, 0.6);
        levels.pump =
            PUMP_LEVEL * fast * fast * wide / (1.0 + depth / PUMP_HALF) * band_gain(pump_hz, 1.5);
        levels.spray = SPRAY_LEVEL * fast.powf(1.5) * wide * depth / (depth + SPRAY_HALF)
            * band_gain(spray_hz, 0.8);

        // Water the tread can't drain is pushed ahead and aside, with power
        // growing with its mass and the speed squared.
        let piled = (depth - tire.drain).max(0.0);
        let power = wide * piled / SPLASH_DEPTH * fast.powi(3);
        let splash = power.sqrt();
        let sheet_hz = SHEET_HZ * fast.powf(0.3);
        self.sheet.tune(sheet_hz, 0.6);
        self.slosh.tune(SLOSH_HZ, 0.8);
        levels.sheet = SHEET_LEVEL * splash * band_gain(sheet_hz, 0.6);
        levels.slosh = SLOSH_LEVEL * splash * band_gain(SLOSH_HZ, 0.8);

        let rise = ((depth - self.last_depth.unwrap_or(depth)) / dt).max(0.0);
        self.last_depth = Some(depth);
        let hit = if depth > tire.drain {
            SLAP_LEVEL * (wide * fast * fast * rise / SLAP_RISE).sqrt() * band_gain(sheet_hz, 0.6)
        } else {
            0.0
        };
        self.slap = (self.slap * self.slap_decay).max(hit);

        self.thrown += (power * DROPS - self.thrown) * dt / FALL;
        if self.dice.unit() < self.thrown * dt {
            self.drop();
        }
        for drop in &mut self.drops {
            drop.block();
        }
    }

    fn drop(&mut self) {
        let Some(drop) = self.drops.iter_mut().find(|drop| drop.level < 1e-3) else {
            return;
        };
        let (smallest, largest) = DROP_RADIUS;
        let radius = smallest + (largest - smallest) * self.dice.unit().powi(2);
        // Minnaert's resonance of a bubble in water.
        let hz = 3.26 / radius;
        let damping = 0.043 * hz + 0.0014 * hz.powf(1.5);
        let size = radius / largest;
        *drop = Drop {
            // Starts at zero, so the drop doesn't click.
            tone: Tone::new(-FRAC_PI_2),
            hz,
            rise: 0.1 * damping * hz,
            level: DROP_LEVEL * size.powf(1.5) * (0.5 + self.dice.unit()),
            decay: (-damping / RATE as f32).exp(),
        };
    }

    pub(super) fn next(&mut self) -> f32 {
        let white = self.rolling.next();
        let levels = &self.levels;
        let mut out =
            self.tread.next(white).band * levels.tread + self.roar.next(white).low * levels.roar;
        let wet = self.water.next();
        out += self.pump.next(white).band * levels.pump;
        if levels.spray > 0.0 {
            out += self.spray.next(wet).band * levels.spray;
        }
        if levels.sheet > 0.0 || self.slap > 1e-4 {
            let sheet = self.sheet.next(wet);
            out += sheet.band * (levels.sheet + self.slap);
            out += self.slosh.next(wet).band * levels.slosh;
        }
        for drop in &mut self.drops {
            if drop.level > 1e-3 {
                out += drop.next();
            }
        }
        out
    }
}

impl Drop {
    fn silent() -> Self {
        Self {
            tone: Tone::new(0.0),
            hz: 0.0,
            rise: 0.0,
            level: 0.0,
            decay: 0.0,
        }
    }

    fn block(&mut self) {
        if self.level > 1e-3 {
            self.hz += self.rise * BLOCK as f32 / RATE as f32;
            self.tone.tune(self.hz);
        }
    }

    fn next(&mut self) -> f32 {
        self.level *= self.decay;
        self.tone.next() * self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEDAN: Tire = Tire {
        width: 0.205,
        load: 1.0,
        pitch: 0.03,
        drain: 2.0,
    };

    /// RMS of the tire at `speed` over `water` millimeters, one per block,
    /// heard from block `from` to `to`.
    fn rms(speed: f32, water: impl Fn(usize) -> f32, from: usize, to: usize) -> f32 {
        let mut sound = SEDAN.sound(1);
        let mut sum = 0.0;
        for block in 0..to {
            sound.block(speed, water(block));
            for _ in 0..BLOCK {
                let sample = sound.next();
                if block >= from {
                    sum += sample * sample;
                }
            }
        }
        (sum / ((to - from) * BLOCK) as f32).sqrt()
    }

    #[test]
    fn deeper_water_is_louder() {
        let second = RATE as usize / BLOCK;
        let dry = rms(11.0, |_| 0.0, 0, second);
        let film = rms(11.0, |_| 0.6, 0, second);
        let puddle = rms(11.0, |_| 15.0, 0, second);
        assert!(film > dry * 1.2, "{dry} {film}");
        assert!(puddle > film * 2.0, "{film} {puddle}");
    }

    #[test]
    fn hitting_a_puddle_splashes() {
        let tenth = RATE as usize / BLOCK / 10;
        let into = |block: usize| if block < tenth { 0.6 } else { 15.0 };
        let wading = rms(11.0, |_| 15.0, tenth, 2 * tenth);
        let hitting = rms(11.0, into, tenth, 2 * tenth);
        assert!(hitting > wading * 1.2, "{wading} {hitting}");
    }

    #[test]
    fn faster_is_louder() {
        let second = RATE as usize / BLOCK;
        let slow = rms(8.0, |_| 0.6, 0, second);
        assert!(rms(13.0, |_| 0.6, 0, second) > slow * 1.5);
    }
}
