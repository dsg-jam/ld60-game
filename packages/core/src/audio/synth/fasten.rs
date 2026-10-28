use std::f32::consts::TAU;

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
/// The hook of a plastic clip springing past its ledge: bright and gone
/// at once.
const CLICK: [Mode; 3] = [
    Mode::new(3_300.0, 2_400.0, 1.0),
    Mode::new(5_400.0, 3_200.0, 1.0),
    Mode::new(8_100.0, 4_500.0, 0.8),
];
/// The hollow case ringing from a click, a short tock.
const SHELL: [Mode; 2] = [
    Mode::new(1_250.0, 260.0, 1.0),
    Mode::new(2_250.0, 420.0, 0.5),
];
/// A clip snaps in a contact this short, which reaches the high modes. So
/// short a contact pushes little, so it pushes harder.
const CLICK_WIDTH: f32 = 0.00012;
const CLICK_LEVEL: f32 = 9.0;
/// A wide plastic part knocking against the case.
const KNOCK: [Mode; 2] = [
    Mode::new(480.0, 600.0, 1.0),
    Mode::new(1_050.0, 900.0, 0.45),
];
/// Steel grinding in a plastic thread.
const GRIND: Rub = Rub {
    modes: &THREAD,
    level: 2.0,
    pulses: 14.0,
};
/// Plastic sliding over pins.
const SLIDE: Rub = Rub {
    modes: &CASE,
    level: 1.5,
    pulses: 0.0,
};
/// The cover flexing a little as its clips give.
const FLEX: Rub = Rub {
    modes: &KNOCK,
    level: 0.15,
    pulses: 35.0,
};

/// How many times the thread catches while a screw turns in or out.
const TURNS: usize = 7;

/// Parts sliding against each other: noise that makes `modes` ring, as
/// loud as `level`, swelling `pulses` times a second, or steady at 0.
struct Rub<'a> {
    modes: &'a [Mode],
    level: f32,
    pulses: f32,
}

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
        Self::fastening(seed, at + 0.08, vary.by(0.05), &ticks, &GRIND)
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
        Self::fastening(seed, at + 0.06, vary.by(0.05), &ticks, &GRIND)
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
            &SLIDE,
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
            &SLIDE,
        )
    }

    /// The back cover lifting off: its two clips let go one after the
    /// other, each with a click and a tock of the case.
    pub(in crate::audio) fn cover_off(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(160));
        let first = 0.012 * vary.by(0.3);
        let second = first + 0.045 * vary.by(0.3);
        Self::fastening(
            seed,
            second + 0.07,
            0.94 * vary.by(0.05),
            &[
                Hit::new(first, CLICK_WIDTH, CLICK_LEVEL, &CLICK),
                Hit::new(first, 0.0003, 0.6, &SHELL),
                Hit::new(second, CLICK_WIDTH, 0.85 * CLICK_LEVEL, &CLICK),
                Hit::new(second, 0.0003, 0.5, &SHELL),
            ],
            &FLEX,
        )
    }

    /// The back cover pressed on: it lands with a soft knock, then its two
    /// clips snap shut close together.
    pub(in crate::audio) fn cover_on(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(170));
        let snap = 0.025 * vary.by(0.3);
        let second = snap + 0.014 * vary.by(0.3);
        Self::fastening(
            seed,
            second + 0.07,
            vary.by(0.05),
            &[
                Hit::new(0.0, 0.001, 0.15, &KNOCK),
                Hit::new(snap, CLICK_WIDTH, CLICK_LEVEL, &CLICK),
                Hit::new(snap, 0.0003, 0.7, &SHELL),
                Hit::new(second, CLICK_WIDTH, 0.9 * CLICK_LEVEL, &CLICK),
                Hit::new(second, 0.0003, 0.6, &SHELL),
            ],
            &FLEX,
        )
    }

    /// `hits` over `seconds`, with all modes tuned by `pitch`, over the
    /// rub of parts sliding against each other until the last hit.
    fn fastening(seed: u32, seconds: f32, pitch: f32, hits: &[Hit], rub: &Rub) -> Self {
        let mut noise = Noise::new(seed.wrapping_add(180));
        let mut grit = Lowpass::new(0.6);
        let mut banks: Vec<_> = hits
            .iter()
            .map(|hit| Resonator::bank(hit.modes, pitch))
            .collect();
        let mut rubbing = Resonator::bank(rub.modes, pitch * 1.03);
        let end = hits.iter().map(|hit| hit.at).fold(0.0, f32::max);
        Self::generate(seconds, |t| {
            // Stick and slip: the rub swells and fades with each turn.
            let swell = if rub.pulses > 0.0 {
                0.35 + 0.65 * (TAU * rub.pulses * t).sin().max(0.0)
            } else {
                1.0
            };
            let drive = if t < end {
                grit.next(noise.next()) * rub.level * swell
            } else {
                0.0
            };
            let rub = rubbing.iter_mut().map(|mode| mode.next(drive)).sum::<f32>();
            hits.iter()
                .zip(&mut banks)
                .map(|(hit, bank)| {
                    let force = Self::pulse(t - hit.at, hit.width) * hit.level;
                    bank.iter_mut().map(|mode| mode.next(force)).sum::<f32>()
                })
                .sum::<f32>()
                + rub
        })
        .normalized(0.8)
    }
}
