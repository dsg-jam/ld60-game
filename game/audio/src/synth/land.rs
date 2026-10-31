//! A small steel part landing on the cutting mat.
//!
//! A screw is too small to ring where it can be heard: its first bending
//! mode lies near 100 kHz. What sounds is the mat and the counter under it.
//! The skin of the mat is stiff plastic over a soft core, so it gives a
//! dull tick with little ring, and the soft core spreads the blow out in
//! time before it reaches the wooden counter.

use super::{Mode, Resonator, Samples, Vary};

/// The wooden counter under the mat, low and short.
const COUNTER: [Mode; 3] = [
    Mode::new(190.0, 160.0, 1.0),
    Mode::new(330.0, 220.0, 0.6),
    Mode::new(560.0, 320.0, 0.35),
];
/// The hard edge of the head or the crest of the thread meeting the skin
/// of the mat first.
const EDGE: [Mode; 2] = [
    Mode::new(6_200.0, 2_600.0, 1.0),
    Mode::new(8_900.0, 3_600.0, 0.6),
];
/// How long the screw presses into the skin, in seconds. By Hertz's
/// contact law, a 0.3 g screw landing at 0.7 m/s with an edge of 0.5 mm
/// radius stays 90 to 220 us on PVC, softer or harder.
const CONTACT: f32 = 0.00015;
/// The first touch of the sharp edge, shorter still.
const TOUCH: f32 = 0.00004;
/// The blow reaches the counter through the soft core, spread over this
/// long. So light a part barely moves the heavy counter.
const THROUGH: f32 = 0.0016;
const EDGE_LEVEL: f32 = 0.12;
const COUNTER_LEVEL: f32 = 0.04;
/// A screw lands on one end and its other end comes down up to this many
/// seconds later, or both at once.
const SECOND_END: f32 = 0.005;
const SECONDS: f32 = 0.08;

impl Samples {
    /// A screw landing on the cutting mat. `seed` varies the mat's modes,
    /// the contact and how the second end follows.
    pub(crate) fn screw_lands(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(190));
        let pitch = vary.by(0.06);
        let skin = Mode::scatter(&mut vary, 1_200.0, 6_000.0, 12, 2.5, 1.0);
        let mut skin = Resonator::even(&skin, pitch);
        let mut edge = Resonator::even(&EDGE, pitch);
        let mut counter = Resonator::even(&COUNTER, vary.by(0.04));
        let contact = CONTACT * vary.by(0.25);
        let second = SECOND_END * (vary.by(1.0) - 1.0).abs();
        let second_level = 0.45 * vary.by(0.6);
        let ring = |bank: &mut [Resonator], force: f32| {
            bank.iter_mut().map(|mode| mode.next(force)).sum::<f32>()
        };
        Self::generate(SECONDS, |t| {
            let ends =
                |width: f32| Self::pulse(t, width) + second_level * Self::pulse(t - second, width);
            ring(&mut skin, ends(contact))
                + EDGE_LEVEL * ring(&mut edge, ends(TOUCH))
                + COUNTER_LEVEL * ring(&mut counter, ends(THROUGH))
        })
        .normalized(0.8)
    }
}
