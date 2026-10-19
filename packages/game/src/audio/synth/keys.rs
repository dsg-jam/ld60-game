use super::{Lowpass, Mode, Noise, Resonator, Samples, Vary};

/// How loud the shop answers a key.
const ROOM: f32 = 0.06;

impl Samples {
    /// A linear key switch going down, after recordings of Cherry MX keys
    /// and Asonov and Agrawal, "Keyboard Acoustic Emanations": the finger
    /// taps the keycap, and a few milliseconds later the stem bottoms out.
    /// The hit rings in the hollow keycap and the plate, thumps in the case
    /// and the desk, rattles the cap and pings the spring.
    pub(in crate::audio) fn key_down(seed: u32) -> Self {
        let mut vary = Vary::new(seed);
        let pitch = vary.by(0.04);
        let touch = 0.12 * vary.by(1.0);
        let bottom = 0.005 * vary.by(0.3);
        let width = 0.00014 * vary.by(0.25);
        let rattle = bottom + 0.0018 * vary.by(0.4);
        let rattle_level = 0.2 * vary.by(0.8);
        let mut cap = Resonator::even(
            &Mode::scatter(&mut vary, 1_000.0, 4_500.0, 8, 25.0, 1.0),
            pitch,
        );
        let mut plate = Resonator::even(
            &Mode::scatter(&mut vary, 3_000.0, 11_000.0, 7, 60.0, 1.0),
            pitch,
        );
        let mut case = Resonator::even(
            &Mode::scatter(&mut vary, 200.0, 650.0, 3, 6.0, 0.4),
            vary.by(0.03),
        );
        let mut desk = Resonator::even(&[Mode::new(110.0 * vary.by(0.1), 120.0, 0.1)], 1.0);
        let mut spring = Resonator::even(
            &[Mode::new(
                4_300.0 * vary.by(0.15),
                30.0,
                0.012 * vary.by(0.6),
            )],
            1.0,
        );
        let mut noise = Noise::new(seed.wrapping_add(40));
        let mut grit = Lowpass::new(0.5);
        let mut pad = Lowpass::new(0.2);
        Self::generate(0.13, |t| {
            let white = noise.next();
            let finger = pad.next(white) * Self::pulse(t, 0.002) * touch;
            let hit = Self::pulse(t - bottom, width);
            let tick = Self::pulse(t - bottom, width * 0.4);
            let force = hit + Self::pulse(t - rattle, width * 0.7) * rattle_level;
            let contact = grit.next(white) * Self::pulse(t - bottom, 0.0008) * 0.4;
            let ring = cap
                .iter_mut()
                .map(|mode| mode.next(force + finger * 0.4))
                .sum::<f32>()
                + plate.iter_mut().map(|mode| mode.next(tick)).sum::<f32>()
                + case.iter_mut().map(|mode| mode.next(force)).sum::<f32>()
                + desk.iter_mut().map(|mode| mode.next(hit)).sum::<f32>()
                + spring.iter_mut().map(|mode| mode.next(hit)).sum::<f32>();
            finger + contact + ring
        })
        .in_room(seed, ROOM)
        .normalized(0.8)
    }

    /// A linear key switch coming back up: the spring throws the stem
    /// against the top of the housing. The hit is smaller and harder than
    /// the press, so it is quieter, higher and shorter, with little of the
    /// case in it. The keycap rattles once on the stem.
    pub(in crate::audio) fn key_up(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(50));
        let pitch = vary.by(0.05);
        let width = 0.00007 * vary.by(0.25);
        let rattle = 0.0012 * vary.by(0.5);
        let rattle_level = 0.3 * vary.by(0.7);
        let mut housing = Resonator::even(
            &Mode::scatter(&mut vary, 2_600.0, 8_000.0, 6, 20.0, 1.0),
            pitch,
        );
        let mut cap = Resonator::even(
            &Mode::scatter(&mut vary, 1_100.0, 4_500.0, 5, 25.0, 0.25),
            pitch * vary.by(0.03),
        );
        let mut case = Resonator::even(&Mode::scatter(&mut vary, 220.0, 650.0, 2, 5.0, 0.08), 1.0);
        let mut spring = Resonator::even(
            &[Mode::new(
                4_300.0 * vary.by(0.15),
                30.0,
                0.02 * vary.by(0.6),
            )],
            1.0,
        );
        let mut noise = Noise::new(seed.wrapping_add(51));
        let mut grit = Lowpass::new(0.7);
        Self::generate(0.09, |t| {
            let hit = Self::pulse(t, width);
            let shake = Self::pulse(t - rattle, width * 1.5) * rattle_level;
            let contact = grit.next(noise.next()) * Self::pulse(t, 0.0004) * 0.15;
            housing.iter_mut().map(|mode| mode.next(hit)).sum::<f32>()
                + cap
                    .iter_mut()
                    .map(|mode| mode.next(hit + shake))
                    .sum::<f32>()
                + case.iter_mut().map(|mode| mode.next(hit)).sum::<f32>()
                + spring.iter_mut().map(|mode| mode.next(hit)).sum::<f32>()
                + contact
        })
        .in_room(seed.wrapping_add(1), ROOM)
        .normalized(0.8)
    }
}
