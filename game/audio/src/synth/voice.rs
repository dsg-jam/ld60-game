//! Locals talking under their breath, in their own language.
//!
//! A source of pulses from the throat goes through three formant filters,
//! as in Klatt, "Software for a cascade/parallel formant synthesizer"
//! (1980), here in parallel. The first two formants glide between vowels
//! from syllable to syllable. Some syllables start with a click of the
//! tongue or a hiss. The pitch falls over the phrase, or rises at its end
//! for a question.

use std::f32::consts::{PI, TAU};

use super::drops::{Bandpass, Dice, Ring, Rings};
use super::{Lowpass, Noise, SAMPLE_RATE, Samples};

/// First and second formants of the vowels, in hertz.
const VOWELS: [(f32, f32); 6] = [
    (700.0, 1_200.0),
    (450.0, 1_900.0),
    (300.0, 2_300.0),
    (450.0, 850.0),
    (320.0, 750.0),
    (550.0, 1_500.0),
];
/// The third formant stays put.
const THIRD: f32 = 2_700.0;
const FORMANT_Q: [f32; 3] = [5.0, 8.0, 10.0];
const FORMANT_LEVEL: [f32; 3] = [1.0, 0.55, 0.25];
/// The filters are retuned this many samples apart.
const RETUNE_EVERY: usize = 32;
const SYLLABLE: (f32, f32) = (0.09, 0.2);
const PAUSE: (f32, f32) = (0.02, 0.07);
const ATTACK: f32 = 0.02;
const RELEASE: f32 = 0.05;
/// Pitch of the voices, low for these large throats.
const PITCH: (f32, f32) = (85.0, 170.0);
/// The pitch wavers this many times per second, by this share.
const WAVER: f32 = 7.0;
const WAVER_DEPTH: f32 = 0.025;
const BREATH: f32 = 0.12;
/// A murmur heard through the rain, dull above this.
const DULL: f32 = 3_000.0;

struct Syllable {
    start: f32,
    end: f32,
    vowel: (f32, f32),
    click: bool,
    hiss: bool,
}

impl Samples {
    /// A short phrase of a few syllables. `seed` picks the voice, the
    /// words and how it is said.
    pub(crate) fn murmur(seed: u32) -> Self {
        let mut dice = Dice::new(seed.wrapping_add(500));
        let mut syllables = Vec::new();
        let mut at = 0.03;
        for _ in 0..dice.between(2.0, 6.0) as usize {
            let length = dice.between(SYLLABLE.0, SYLLABLE.1);
            let vowel = VOWELS[(dice.unit() * VOWELS.len() as f32) as usize % VOWELS.len()];
            syllables.push(Syllable {
                start: at,
                end: at + length,
                vowel,
                click: dice.chance(0.3),
                hiss: dice.chance(0.2),
            });
            at += length + dice.between(PAUSE.0, PAUSE.1);
        }
        let length = at;
        let base = dice.spread(PITCH.0, PITCH.1);
        let question = dice.chance(0.3);
        let mut rings = Rings::default();
        let mut formants = [
            Bandpass::new(syllables[0].vowel.0, FORMANT_Q[0]),
            Bandpass::new(syllables[0].vowel.1, FORMANT_Q[1]),
            Bandpass::new(THIRD, FORMANT_Q[2]),
        ];
        let mut hiss = Bandpass::new(4_500.0, 2.0);
        let mut noise = Noise::new(seed.wrapping_add(501));
        let mut dull = Lowpass::at(DULL);
        let mut phase = 0.0f32;
        let mut index = 0usize;
        let mut clicked = 0usize;
        Self::generate(length + RELEASE, |t| {
            while index + 1 < syllables.len() && t >= syllables[index].end {
                index += 1;
            }
            let syllable = &syllables[index];
            if clicked <= index && t >= syllable.start {
                clicked = index + 1;
                if syllable.click {
                    rings.add(Ring::impact(dice.spread(1_800.0, 3_500.0), 0.35));
                }
            }
            let sample = (t * SAMPLE_RATE as f32) as usize;
            if sample.is_multiple_of(RETUNE_EVERY) {
                let next = syllables.get(index + 1).unwrap_or(syllable);
                let along =
                    ((t - syllable.start) / (syllable.end - syllable.start)).clamp(0.0, 1.0);
                // Glide into the next vowel over the end of the syllable.
                let glide = ((along - 0.6) / 0.4).clamp(0.0, 1.0);
                let first = syllable.vowel.0 + (next.vowel.0 - syllable.vowel.0) * glide;
                let second = syllable.vowel.1 + (next.vowel.1 - syllable.vowel.1) * glide;
                formants[0].retune(first, FORMANT_Q[0]);
                formants[1].retune(second, FORMANT_Q[1]);
            }
            let over = t / length;
            let contour = if question && over > 0.7 {
                1.0 + (over - 0.7) * 1.2
            } else {
                1.1 - 0.2 * over
            };
            let pitch = base * contour * (1.0 + WAVER_DEPTH * (TAU * WAVER * t).sin());
            phase = (phase + pitch / SAMPLE_RATE as f32).fract();
            // A pulse from the throat, rich in harmonics, and some breath.
            let pulse = (PI * phase).sin().powi(8) - 0.25;
            let breath = noise.next();
            let source = pulse + BREATH * breath;
            let envelope = Self::envelope(t, syllable);
            let voiced: f32 = formants
                .iter_mut()
                .zip(FORMANT_LEVEL)
                .map(|(formant, level)| formant.next(source) * level)
                .sum();
            let hissed = if syllable.hiss && t < syllable.start + 0.05 && t >= syllable.start {
                hiss.next(breath) * 0.6
            } else {
                hiss.next(0.0)
            };
            dull.next(voiced * envelope * 4.0 + hissed) + rings.next()
        })
        .normalized(0.7)
    }

    /// How loud a syllable is at `t`: it swells in and dies away.
    fn envelope(t: f32, syllable: &Syllable) -> f32 {
        let into = t - syllable.start;
        let left = syllable.end + RELEASE - t;
        if into < 0.0 || left < 0.0 {
            return 0.0;
        }
        let rise = (into / ATTACK).min(1.0);
        let fall = (left / RELEASE).min(1.0);
        (rise * fall * PI / 2.0).sin()
    }
}
