use super::{Lowpass, Mode, Noise, Resonator, Samples, Vary};

/// The sheet landing on the pages under it, a soft low thump.
const FLAP: [Mode; 2] = [Mode::new(160.0, 60.0, 1.0), Mode::new(310.0, 110.0, 0.4)];

impl Samples {
    /// A page turning: the sheet bends and rustles with small crinkles as
    /// it lifts and sweeps over, then flaps down on the pages under it.
    pub(in crate::audio) fn page_turn(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(60));
        let land = 0.3 * vary.by(0.15);
        let seconds = land + 0.2;
        let crinkles = 0.004 * vary.by(0.4);
        let mut noise = Noise::new(seed.wrapping_add(61));
        let mut dice = Noise::new(seed.wrapping_add(62));
        let mut hiss = Lowpass::new(0.25);
        let mut soft = Lowpass::new(0.5);
        let mut softer = Lowpass::new(0.5);
        let mut air = Lowpass::new(0.12);
        let mut air_low = Lowpass::new(0.02);
        let mut thud = Lowpass::new(0.02);
        let mut flap = Resonator::even(&FLAP, vary.by(0.1));
        let mut crinkle = 0.0_f32;
        Self::generate(seconds, |t| {
            let white = noise.next();
            // Lifts quickly, sweeps over and slows down before it lands.
            let sweep = Self::swell(t, 0.04, land);
            if (dice.next() + 1.0) / 2.0 < crinkles * sweep {
                crinkle += 0.5 + 0.5 * (dice.next() + 1.0) / 2.0;
            }
            crinkle *= 0.992;
            let bright = softer.next(soft.next(white - hiss.next(white))) * 1.5;
            let rustle = bright * (0.15 + crinkle) * sweep;
            let band = air.next(white);
            let whoosh = (band - air_low.next(band)) * sweep * 0.6;
            let landed = t - land;
            let landing = if landed < 0.0 {
                0.0
            } else {
                thud.next(white) * 0.8 * (-landed * 40.0).exp()
                    + bright * 0.4 * (-landed * 250.0).exp()
            };
            let force = Self::pulse(landed, 0.006);
            let body = flap.iter_mut().map(|mode| mode.next(force)).sum::<f32>();
            rustle + whoosh + landing + body * 0.002
        })
        .normalized(0.7)
    }

    /// Rises over `attack` seconds and falls smoothly to 0 at `end`.
    fn swell(t: f32, attack: f32, end: f32) -> f32 {
        if t >= end {
            0.0
        } else if t < attack {
            t / attack
        } else {
            let fall = (t - attack) / (end - attack);
            1.0 - fall * fall
        }
    }
}
