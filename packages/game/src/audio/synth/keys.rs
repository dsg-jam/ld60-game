use super::{Lowpass, Mode, Noise, Resonator, Samples, Vary};

/// The keycap and the plate: stiff plastic and steel, high and short.
const CAP: [Mode; 4] = [
    Mode::new(1_350.0, 260.0, 1.0),
    Mode::new(2_450.0, 380.0, 0.8),
    Mode::new(3_900.0, 550.0, 0.5),
    Mode::new(6_300.0, 850.0, 0.25),
];
/// The air and the case under the plate, which give the low thock.
const CASE: [Mode; 2] = [Mode::new(420.0, 90.0, 0.15), Mode::new(780.0, 160.0, 0.15)];
/// The steel spring, a faint ping that rings on.
const SPRING: [Mode; 2] = [
    Mode::new(5_200.0, 30.0, 0.02),
    Mode::new(7_900.0, 45.0, 0.012),
];
/// The slider hitting the top housing on the way up: smaller and brighter.
const TOP: [Mode; 3] = [
    Mode::new(2_900.0, 450.0, 1.0),
    Mode::new(4_700.0, 650.0, 0.6),
    Mode::new(7_400.0, 900.0, 0.3),
];

impl Samples {
    /// A linear key switch going down, after recordings of Cherry MX keys:
    /// the stem slides with a faint scrape, then bottoms out. The hit rings
    /// in the keycap and plate, thumps in the case and pings the spring.
    pub(in crate::audio) fn key_down(seed: u32) -> Self {
        let mut vary = Vary::new(seed);
        let pitch = vary.by(0.04);
        let bottom = 0.004 * vary.by(0.25);
        let rock = bottom + 0.0012 * vary.by(0.3);
        let rock_level = 0.3 * vary.by(0.5);
        let mut noise = Noise::new(seed.wrapping_add(40));
        let mut scrape = Lowpass::new(0.3);
        let mut cap = Resonator::even(&CAP, pitch);
        let mut case = Resonator::even(&CASE, pitch * vary.by(0.05));
        let mut spring = Resonator::even(&SPRING, vary.by(0.03));
        Self::generate(0.14, |t| {
            let white = noise.next();
            let slide = if t < bottom {
                (white - scrape.next(white)) * 0.01 * t / bottom
            } else {
                0.0
            };
            let force =
                Self::pulse(t - bottom, 0.00012) + Self::pulse(t - rock, 0.0001) * rock_level;
            let click = white * Self::pulse(t - bottom, 0.0008) * 0.15;
            let ring = cap.iter_mut().map(|mode| mode.next(force)).sum::<f32>()
                + case.iter_mut().map(|mode| mode.next(force)).sum::<f32>()
                + spring.iter_mut().map(|mode| mode.next(force)).sum::<f32>();
            slide + click + ring
        })
        .normalized(0.8)
    }

    /// A linear key switch coming back up: the slider tops out against the
    /// housing, softer and brighter than the press, and the spring rings a
    /// little.
    pub(in crate::audio) fn key_up(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(50));
        let pitch = vary.by(0.05);
        let mut noise = Noise::new(seed.wrapping_add(51));
        let mut top = Resonator::even(&TOP, pitch);
        let mut cap = Resonator::even(&CAP, pitch * 1.1);
        let mut spring = Resonator::even(&SPRING, vary.by(0.03));
        Self::generate(0.1, |t| {
            let force = Self::pulse(t, 0.0001);
            let click = noise.next() * Self::pulse(t, 0.0003) * 0.03;
            top.iter_mut().map(|mode| mode.next(force)).sum::<f32>()
                + cap
                    .iter_mut()
                    .map(|mode| mode.next(force * 0.3))
                    .sum::<f32>()
                + spring
                    .iter_mut()
                    .map(|mode| mode.next(force * 0.4))
                    .sum::<f32>()
                + click
        })
        .normalized(0.35)
    }
}
