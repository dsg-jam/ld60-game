use std::f32::consts::TAU;

use super::dsp::{Glide, Noise, Svf, Tone, band_gain};

/// Power for the drag, against the power to hover, per cubed meter per
/// second. The fans spin half as fast again at 45 m/s.
const DRAG: f32 = 2.6e-5;
/// The two fans are a little out of tune, so they beat.
const DETUNE: f32 = 1.004;
const RUSH_HZ: f32 = 1_200.0;
const TURBINE: f32 = 6.0;
/// The wind around the body is as loud as its level at this speed, in
/// meters per second, and its pitch grows with the speed.
const WIND_SPEED: f32 = 30.0;
const WIND_HZ: f32 = 25.0;
const WASH_HZ: f32 = 120.0;
/// Hertz of the gusts in the air the fans pull in.
const GUSTS_HZ: f32 = 3.0;
const GUSTS: f32 = 0.2;
const GLIDE: f32 = 0.2;

const TONE_LEVEL: f32 = 0.5;
const RUSH_LEVEL: f32 = 0.6;
const TURBINE_LEVEL: f32 = 0.04;
const WIND_LEVEL: f32 = 0.5;
const WASH_LEVEL: f32 = 0.4;

/// The ducted lift fans of a flying car.
#[derive(Clone, Copy, Debug)]
pub(super) struct Fans {
    /// Hertz at which blades pass while the car hovers.
    pub(super) blades: f32,
    pub(super) level: f32,
}

/// The lift fans and the wind of a flying car. The faster it flies, the
/// more power the fans need against the drag, so they spin faster, higher
/// and louder, with their noise growing with the fifth power of the tip
/// speed. The wind around the body rises steeply and brightens with the
/// speed.
pub(super) struct FanSound {
    fans: Fans,
    speed: Glide,
    tones: [Tone; 3],
    noise: Noise,
    rush: Svf,
    wind: Svf,
    wash: Svf,
    gusts: Svf,
    tone_level: f32,
    rush_level: f32,
    wind_level: f32,
    wash_level: f32,
}

impl Fans {
    pub(super) fn sound(self, seed: u32) -> FanSound {
        let mut noise = Noise::new(seed.wrapping_add(20));
        let mut gusts = Svf::default();
        gusts.tune(GUSTS_HZ, 0.7);
        FanSound {
            fans: self,
            speed: Glide::from_first(GLIDE),
            tones: std::array::from_fn(|_| Tone::new(noise.unit() * TAU)),
            noise,
            rush: Svf::default(),
            wind: Svf::default(),
            wash: Svf::default(),
            gusts,
            tone_level: 0.0,
            rush_level: 0.0,
            wind_level: 0.0,
            wash_level: 0.0,
        }
    }
}

impl FanSound {
    pub(super) fn block(&mut self, speed: f32) {
        let fans = self.fans;
        let speed = self.speed.follow(speed).max(0.0);
        let spin = (1.0 + DRAG * speed.powi(3)).cbrt();
        let blades = fans.blades * spin;
        self.tones[0].tune(blades);
        self.tones[1].tune(blades * DETUNE);
        self.tones[2].tune(blades * TURBINE);
        let rush_hz = RUSH_HZ * spin;
        self.rush.tune(rush_hz, 0.7);
        let wind_hz = WIND_HZ * speed;
        self.wind.tune(wind_hz, 0.6);
        self.wash.tune(WASH_HZ, 0.7);
        let loud = fans.level * spin.powf(2.5);
        self.tone_level = TONE_LEVEL * loud;
        self.rush_level = RUSH_LEVEL * loud * band_gain(rush_hz, 0.7);
        self.wash_level = WASH_LEVEL * loud * band_gain(WASH_HZ, 0.6);
        self.wind_level =
            WIND_LEVEL * fans.level * (speed / WIND_SPEED).powi(3) * band_gain(wind_hz, 0.6);
    }

    pub(super) fn next(&mut self) -> f32 {
        let white = self.noise.next();
        let gust = 1.0 + GUSTS * self.gusts.next(white).low * band_gain(GUSTS_HZ, 0.6);
        let mut tones = 0.0;
        for tone in &mut self.tones[..2] {
            let x = tone.next();
            tones += x + 0.5 * (2.0 * x * x - 1.0) + 0.3 * (4.0 * x * x - 3.0) * x;
        }
        let turbine = self.tones[2].next() * TURBINE_LEVEL / TONE_LEVEL;
        let rush = self.rush.next(white).band * self.rush_level;
        let wind = self.wind.next(white).band * self.wind_level;
        let wash = self.wash.next(white).low * self.wash_level;
        ((tones + turbine) * self.tone_level + rush) * gust + wind + wash
    }
}
