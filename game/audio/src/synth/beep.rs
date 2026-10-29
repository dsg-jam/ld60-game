use super::{Lowpass, Samples};

/// The two notes of the beep, each with its start and length in seconds.
const NOTES: [(f32, f32, f32); 2] = [(392.0, 0.0, 0.11), (294.0, 0.14, 0.16)];
/// A small speaker in a plastic case, which rounds off the square wave.
const SPEAKER_HZ: f32 = 1_800.0;
/// Seconds for each note to come up and die away, so it does not click.
const EDGE: f32 = 0.006;

impl Samples {
    /// An old computer saying no: two short square wave beeps, the second
    /// lower, from a small speaker.
    pub(crate) fn refuse() -> Self {
        let mut speaker = Lowpass::at(SPEAKER_HZ);
        let end = NOTES[1].1 + NOTES[1].2 + 0.05;
        Self::generate(end, |t| {
            let tone: f32 = NOTES
                .iter()
                .map(|&(frequency, start, length)| {
                    let local = t - start;
                    if !(0.0..length).contains(&local) {
                        return 0.0;
                    }
                    let level = (local / EDGE).min((length - local) / EDGE).min(1.0);
                    let square = if (frequency * local).fract() < 0.5 {
                        1.0
                    } else {
                        -1.0
                    };
                    square * level
                })
                .sum();
            speaker.next(tone)
        })
        .normalized(0.5)
    }
}
