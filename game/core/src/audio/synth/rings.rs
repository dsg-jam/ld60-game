use std::f32::consts::TAU;

use super::{Mode, Noise, Resonator, Room, Samples, Vary};
use crate::layout::desk::DeskSpots;

/// Meters from the eyes to the manuals file while reading it.
const REACH: f32 = 0.33;
const SECONDS: f32 = 0.3;
/// The steel halves of a ring, which ring high and long.
const RING: [Mode; 3] = [
    Mode::new(3_400.0, 45.0, 1.0),
    Mode::new(5_900.0, 70.0, 0.6),
    Mode::new(8_800.0, 110.0, 0.35),
];
/// The plate of the mechanism and its lever, duller and short.
const PLATE: [Mode; 2] = [
    Mode::new(1_500.0, 450.0, 1.0),
    Mode::new(2_700.0, 650.0, 0.5),
];
/// The spring under the plate, which twangs down to this pitch in hertz.
const SPRING: f32 = 620.0;

impl Samples {
    /// The lever of the binder flipping down: the plate knocks, the spring
    /// lets go with a twang and the halves of the rings spring apart.
    pub(in crate::audio) fn rings_open(seed: u32) -> Self {
        Self::snap(seed, false)
    }

    /// The rings of the binder snapping shut: the halves clack together and
    /// the lever and the spring settle.
    pub(in crate::audio) fn rings_shut(seed: u32) -> Self {
        Self::snap(seed.wrapping_add(300), true)
    }

    fn snap(seed: u32, shut: bool) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(70));
        let pitch = vary.by(0.05);
        let late = 0.007 * vary.by(0.3);
        // Each contact: when, how long, how hard and what it rings.
        let hits: [(f32, f32, f32, &[Mode]); 2] = if shut {
            [(0.0, 0.0003, 1.0, &RING), (late, 0.0008, 0.5, &PLATE)]
        } else {
            [(0.0, 0.0008, 1.0, &PLATE), (late, 0.0003, 0.6, &RING)]
        };
        let mut banks: Vec<_> = hits
            .iter()
            .map(|(.., modes)| Resonator::bank(modes, pitch))
            .collect();
        let mut noise = Noise::new(seed.wrapping_add(71));
        let spring = SPRING * vary.by(0.1);
        let twang = if shut { 0.06 } else { 0.12 };
        Self::generate(SECONDS, |t| {
            let contacts: f32 = hits
                .iter()
                .zip(&mut banks)
                .map(|(&(at, width, level, _), bank)| {
                    let force = Self::pulse(t - at, width) * level;
                    bank.iter_mut().map(|mode| mode.next(force)).sum::<f32>()
                })
                .sum();
            let since = (t - late).max(0.0);
            // The spring starts a little sharp and drops to its pitch.
            let phase = TAU * spring * since * (1.0 + 0.4 * (-since * 25.0).exp());
            let twang = phase.sin() * (-since * 28.0).exp() * twang * (since * 2_000.0).min(1.0);
            let scrape = noise.next() * (-t * 3_000.0).exp() * 0.03;
            contacts + twang + scrape
        })
        .in_room(seed, &{
            let spots = DeskSpots::new();
            Room::leaning_in(spots.manuals, spots.seat, REACH)
        })
        .normalized(0.7)
    }
}
