use super::{Hit, Lowpass, Mode, Noise, Resonator, Samples, Vary};

/// The thread of a small steel screw catching, high and short.
const THREAD: [Mode; 3] = [
    Mode::new(2_700.0, 900.0, 1.0),
    Mode::new(4_300.0, 1_300.0, 0.6),
    Mode::new(6_900.0, 1_900.0, 0.3),
];
/// The plastic case the screw turns in, dull and quick.
const CASE: [Mode; 2] = [
    Mode::new(700.0, 900.0, 1.0),
    Mode::new(1_350.0, 1_200.0, 0.5),
];
/// The catch of a plastic connector or cover snapping over its stop.
const SNAP: [Mode; 2] = [
    Mode::new(2_100.0, 1_400.0, 1.0),
    Mode::new(3_700.0, 1_900.0, 0.5),
];
/// A wide plastic part knocking against the case.
const KNOCK: [Mode; 2] = [
    Mode::new(480.0, 600.0, 1.0),
    Mode::new(1_050.0, 900.0, 0.45),
];
/// How many times the thread catches while a screw turns in or out.
const TURNS: usize = 7;

impl Samples {
    /// A screw turned out: the thread ticks, quicker as it loosens, and the
    /// screw comes free with a light knock.
    pub(in crate::audio) fn screw_out(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(110));
        let mut at = 0.0;
        let mut ticks = Vec::with_capacity(TURNS + 1);
        for turn in 0..TURNS {
            ticks.push(Hit::new(at, 0.0004, 0.5 + 0.3 * vary.by(0.5), &THREAD));
            at += 0.06 * (1.0 - turn as f32 / (2.0 * TURNS as f32)) * vary.by(0.15);
        }
        ticks.push(Hit::new(at + 0.02, 0.0008, 0.8, &CASE));
        Self::fastening(seed, at + 0.08, vary.by(0.05), &ticks, 0.6)
    }

    /// A screw turned in: the thread ticks, slower as it tightens, and it
    /// stops hard against the case.
    pub(in crate::audio) fn screw_in(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(120));
        let mut at = 0.0;
        let mut ticks = Vec::with_capacity(TURNS + 1);
        for turn in 0..TURNS {
            ticks.push(Hit::new(at, 0.0004, 0.5 + 0.3 * vary.by(0.5), &THREAD));
            at += 0.045 * (1.0 + turn as f32 / TURNS as f32) * vary.by(0.15);
        }
        ticks.push(Hit::new(at, 0.0006, 1.0, &CASE));
        ticks.push(Hit::new(at + 0.002, 0.0003, 0.7, &THREAD));
        Self::fastening(seed, at + 0.06, vary.by(0.05), &ticks, 0.8)
    }

    /// The debug plug pushed onto the header: the plastic slides over the
    /// pins, then snaps home.
    pub(in crate::audio) fn plug_in(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(140));
        let home = 0.05 * vary.by(0.2);
        Self::fastening(
            seed,
            home + 0.05,
            vary.by(0.05),
            &[
                Hit::new(home, 0.0004, 1.0, &SNAP),
                Hit::new(home + 0.003, 0.0008, 0.6, &KNOCK),
            ],
            0.5,
        )
    }

    /// The debug plug pulled off: it gives with a pop and slides free.
    pub(in crate::audio) fn plug_out(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(150));
        Self::fastening(
            seed,
            0.09,
            vary.by(0.05) * 1.1,
            &[
                Hit::new(0.0, 0.0003, 0.8, &SNAP),
                Hit::new(0.03 * vary.by(0.2), 0.0005, 0.25, &KNOCK),
            ],
            0.35,
        )
    }

    /// The back cover lifting off: its catches let go one after the other.
    pub(in crate::audio) fn cover_off(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(160));
        let second = 0.04 * vary.by(0.3);
        Self::fastening(
            seed,
            second + 0.08,
            vary.by(0.05),
            &[
                Hit::new(0.0, 0.0004, 0.8, &SNAP),
                Hit::new(second, 0.0004, 0.6, &SNAP),
                Hit::new(second + 0.025, 0.001, 0.5, &KNOCK),
            ],
            0.2,
        )
    }

    /// The back cover pressed on: it knocks down and its catches snap shut.
    pub(in crate::audio) fn cover_on(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(170));
        let snap = 0.02 * vary.by(0.3);
        Self::fastening(
            seed,
            snap + 0.07,
            vary.by(0.05),
            &[
                Hit::new(0.0, 0.001, 0.7, &KNOCK),
                Hit::new(snap, 0.0004, 1.0, &SNAP),
                Hit::new(snap + 0.012 * vary.by(0.3), 0.0004, 0.8, &SNAP),
            ],
            0.1,
        )
    }

    /// `hits` over `seconds`, with all modes tuned by `pitch`, and a soft rub
    /// of parts sliding against each other as loud as `rub`.
    fn fastening(seed: u32, seconds: f32, pitch: f32, hits: &[Hit], rub: f32) -> Self {
        let mut noise = Noise::new(seed.wrapping_add(180));
        let mut hiss = Lowpass::new(0.25);
        let mut banks: Vec<_> = hits
            .iter()
            .map(|hit| Resonator::bank(hit.modes, pitch))
            .collect();
        let end = hits.iter().map(|hit| hit.at).fold(0.0, f32::max);
        Self::generate(seconds, |t| {
            let sliding = if t < end { 1.0 } else { 0.0 };
            let rubbing = hiss.next(noise.next()) * rub * 0.03 * sliding;
            hits.iter()
                .zip(&mut banks)
                .map(|(hit, bank)| {
                    let force = Self::pulse(t - hit.at, hit.width) * hit.level;
                    bank.iter_mut().map(|mode| mode.next(force)).sum::<f32>()
                })
                .sum::<f32>()
                + rubbing
        })
        .normalized(0.8)
    }
}
