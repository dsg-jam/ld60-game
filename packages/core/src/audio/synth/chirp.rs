use std::f32::consts::TAU;

use super::Samples;

/// The notes of the chirp, each with its start and length in seconds and
/// the pitch it slides from and to.
const NOTES: [(f32, f32, f32, f32); 3] = [
    (0.0, 0.07, 1_800.0, 2_400.0),
    (0.09, 0.07, 2_100.0, 2_800.0),
    (0.18, 0.12, 2_600.0, 3_300.0),
];

impl Samples {
    /// A happy little chirp of three rising notes, like a small machine
    /// that has news.
    pub(in crate::audio) fn chirp() -> Self {
        Self::generate(0.34, |t| {
            NOTES
                .iter()
                .filter(|(start, length, ..)| t >= *start && t < start + length)
                .map(|&(start, length, from, to)| {
                    let t = t - start;
                    let along = t / length;
                    // The phase of a pitch that slides from `from` to `to`.
                    let phase = TAU * (from * t + (to - from) * t * along / 2.0);
                    let swell = (along * std::f32::consts::PI).sin();
                    phase.sin() * swell
                })
                .sum()
        })
        .normalized(0.5)
    }
}
