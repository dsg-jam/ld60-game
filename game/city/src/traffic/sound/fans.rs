use std::f32::consts::TAU;

use super::dsp::{Glide, Noise, Svf, Tone, Wander, band_gain};

/// Power for the drag, against the power to hover, per cubed meter per
/// second. The fans spin half as fast again at 45 m/s.
const DRAG: f32 = 2.6e-5;
/// The two fans are a little out of tune and wander in speed on their own,
/// so they beat slowly and unevenly.
const DETUNE: f32 = 1.003;
const WANDER: f32 = 0.006;
const WANDER_HZ: f32 = 0.3;
/// The turbulent air the fans pull in shakes the load on the blades, which
/// smears their tone into a band.
const LOADING_HZ: f32 = 6.0;
const LOADING: f32 = 0.4;
const HUM_Q: f32 = 10.0;
const RUSH_HZ: f32 = 700.0;
const RUSH_Q: f32 = 0.5;
/// The motors sing softly this many times above the blades.
const TURBINE: f32 = 4.0;
const TURBINE_Q: f32 = 10.0;
/// The ducts let little through above this, in hertz.
const DUCT_HZ: f32 = 1_400.0;
/// The wind around the body is as loud as its level at this speed, in
/// meters per second, and its pitch grows with the speed.
const WIND_SPEED: f32 = 30.0;
const WIND_HZ: f32 = 16.0;
const WASH_HZ: f32 = 120.0;
/// Hertz of the gusts in the air the fans pull in.
const GUSTS_HZ: f32 = 3.0;
const GUSTS: f32 = 0.2;
const GLIDE: f32 = 0.2;

const TONE_LEVEL: f32 = 0.2;
const SECOND_FAN: f32 = 0.4;
const HUM_LEVEL: f32 = 0.35;
const OVERTONE_LEVEL: f32 = 0.08;
const TURBINE_LEVEL: f32 = 0.012;
const RUSH_LEVEL: f32 = 0.9;
const WIND_LEVEL: f32 = 0.4;
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
/// speed. The blades hum in a band around their passing tone rather than
/// on one pitch, under a broad rush of air. The wind around the body rises
/// steeply and brightens with the speed.
pub(super) struct FanSound {
    fans: Fans,
    speed: Glide,
    tones: [Tone; 2],
    wanders: [Wander; 2],
    noise: Noise,
    hum: Svf,
    overtone: Svf,
    turbine: Svf,
    loading: Svf,
    rush: Svf,
    duct: Svf,
    wind: Svf,
    wash: Svf,
    gusts: Svf,
    tone_level: f32,
    hum_level: f32,
    overtone_level: f32,
    turbine_level: f32,
    rush_level: f32,
    wind_level: f32,
    wash_level: f32,
}

impl Fans {
    pub(super) fn sound(self, seed: u32) -> FanSound {
        let mut noise = Noise::new(seed.wrapping_add(20));
        let mut gusts = Svf::default();
        gusts.tune(GUSTS_HZ, 0.7);
        let mut loading = Svf::default();
        loading.tune(LOADING_HZ, 0.7);
        let mut wash = Svf::default();
        wash.tune(WASH_HZ, 0.7);
        FanSound {
            fans: self,
            speed: Glide::from_first(GLIDE),
            tones: std::array::from_fn(|_| Tone::new(noise.unit() * TAU)),
            wanders: std::array::from_fn(|fan| Wander::new(WANDER_HZ, seed ^ (fan as u32 + 1))),
            noise,
            hum: Svf::default(),
            overtone: Svf::default(),
            turbine: Svf::default(),
            loading,
            rush: Svf::default(),
            duct: Svf::default(),
            wind: Svf::default(),
            wash,
            gusts,
            tone_level: 0.0,
            hum_level: 0.0,
            overtone_level: 0.0,
            turbine_level: 0.0,
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
        let [first, second] = self
            .wanders
            .each_mut()
            .map(|wander| 1.0 + WANDER * wander.next());
        self.tones[0].tune(blades * first);
        self.tones[1].tune(blades * second * DETUNE);
        self.hum.tune(blades, HUM_Q);
        self.overtone.tune(2.0 * blades, HUM_Q);
        self.turbine.tune(TURBINE * blades, TURBINE_Q);
        let rush_hz = RUSH_HZ * spin;
        self.rush.tune(rush_hz, RUSH_Q);
        self.duct.tune(DUCT_HZ * spin.sqrt(), 0.6);
        let wind_hz = WIND_HZ * speed;
        self.wind.tune(wind_hz, 0.6);
        let loud = fans.level * spin.powf(2.5);
        self.tone_level = TONE_LEVEL * loud;
        self.hum_level = HUM_LEVEL * loud * band_gain(blades, HUM_Q);
        self.overtone_level = OVERTONE_LEVEL * loud * band_gain(2.0 * blades, HUM_Q);
        self.turbine_level = TURBINE_LEVEL * loud * band_gain(TURBINE * blades, TURBINE_Q);
        self.rush_level = RUSH_LEVEL * loud * band_gain(rush_hz, RUSH_Q);
        self.wash_level = WASH_LEVEL * loud * band_gain(WASH_HZ, 0.6);
        self.wind_level =
            WIND_LEVEL * fans.level * (speed / WIND_SPEED).powi(3) * band_gain(wind_hz, 0.6);
    }

    pub(super) fn next(&mut self) -> f32 {
        let white = self.noise.next();
        let shake = self.noise.next();
        let gust = 1.0 + GUSTS * self.gusts.next(white).low * band_gain(GUSTS_HZ, 0.6);
        let load = 1.0 + LOADING * self.loading.next(shake).low * band_gain(LOADING_HZ, 0.6);
        let tones = self.tones[0].next() + SECOND_FAN * self.tones[1].next();
        let hum = self.hum.next(shake).band * self.hum_level
            + self.overtone.next(shake).band * self.overtone_level;
        let blades = (tones * self.tone_level + hum) * load;
        let turbine = self.turbine.next(white).band * self.turbine_level;
        let rush = self.rush.next(white).band * self.rush_level;
        let fans = self.duct.next((blades + turbine + rush) * gust).low;
        let wind = self.wind.next(white).band * self.wind_level;
        let wash = self.wash.next(white).low * self.wash_level;
        fans + wind + wash
    }
}
