use ld60_game_core::layout::desk::DeskSpots;

use super::{Lowpass, Mode, Noise, Resonator, Room, Samples, Vary};

/// All keys share the same caps, plate, case and springs, so they ring at
/// the same frequencies. Only how a key is hit differs from press to press.
const KEYBOARD: u32 = 7;
/// Meters from the eyes to the keyboard while sitting at the computer.
const REACH: f32 = 0.3;
/// The steel spring, a faint short ping, damped by the stem around it.
const SPRING: [Mode; 1] = [Mode::new(4_300.0, 90.0, 0.006)];

impl Samples {
    /// A linear key switch going down, after recordings of Cherry MX keys
    /// and Asonov and Agrawal, "Keyboard Acoustic Emanations": the finger
    /// taps the keycap, and a few milliseconds later the stem bottoms out.
    /// The hit rings in the hollow keycap and the plate, thumps in the case
    /// and the desk, rattles the cap and pings the spring.
    pub(crate) fn key_down(seed: u32) -> Self {
        let mut build = Vary::new(KEYBOARD);
        let mut vary = Vary::new(seed);
        let touch = 0.12 * vary.by(1.0);
        let bottom = 0.005 * vary.by(0.3);
        let width = 0.00014 * vary.by(0.25);
        let rattle = bottom + 0.0018 * vary.by(0.4);
        let rattle_level = 0.2 * vary.by(0.8);
        let mut cap = Resonator::even(
            &Mode::scatter(&mut build, 1_000.0, 4_500.0, 10, 8.0, 1.0),
            1.0,
        );
        let mut plate = Resonator::even(
            &Mode::scatter(&mut build, 3_000.0, 11_000.0, 9, 25.0, 1.0),
            1.0,
        );
        let mut case = Resonator::even(&Mode::scatter(&mut build, 200.0, 650.0, 3, 6.0, 0.4), 1.0);
        let mut desk = Resonator::even(&[Mode::new(110.0, 120.0, 0.1)], 1.0);
        let mut spring = Resonator::even(&SPRING, 1.0);
        Self::scuff(&mut vary, [&mut cap, &mut plate, &mut case]);
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
        .in_room(seed, &Self::keyboard_room())
        .normalized(0.8)
    }

    /// A linear key switch coming back up: the spring throws the stem
    /// against the top of the housing. The hit is smaller and harder than
    /// the press, so it is quieter, higher and shorter, with little of the
    /// case in it. The keycap rattles once on the stem.
    pub(crate) fn key_up(seed: u32) -> Self {
        let mut build = Vary::new(KEYBOARD.wrapping_add(1));
        let mut vary = Vary::new(seed.wrapping_add(50));
        let width = 0.00007 * vary.by(0.25);
        let rattle = 0.0012 * vary.by(0.5);
        let rattle_level = 0.3 * vary.by(0.7);
        let mut housing = Resonator::even(
            &Mode::scatter(&mut build, 2_600.0, 8_000.0, 8, 10.0, 1.0),
            1.0,
        );
        let mut cap = Resonator::even(
            &Mode::scatter(&mut build, 1_100.0, 4_500.0, 6, 8.0, 0.25),
            1.0,
        );
        let mut case = Resonator::even(&Mode::scatter(&mut build, 220.0, 650.0, 2, 5.0, 0.08), 1.0);
        let mut spring = Resonator::even(&SPRING, 1.0);
        Self::scuff(&mut vary, [&mut housing, &mut cap, &mut case]);
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
        .in_room(seed.wrapping_add(1), &Self::keyboard_room())
        .normalized(0.8)
    }

    /// Each press hits the parts a little differently, so their modes ring
    /// louder or softer, while their pitch stays.
    fn scuff<const N: usize>(vary: &mut Vary, banks: [&mut Vec<Resonator>; N]) {
        for bank in banks {
            for mode in bank {
                mode.gain *= vary.by(0.4);
            }
        }
    }

    fn keyboard_room() -> Room {
        let spots = DeskSpots::new();
        Room::leaning_in(spots.computer, spots.seat, REACH)
    }
}
