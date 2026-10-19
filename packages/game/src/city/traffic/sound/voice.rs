use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use bevy::{
    audio::{ChannelCount, Decodable, Sample, SampleRate},
    prelude::*,
};

use super::dsp::{Glide, one_pole};
use super::fans::{FanSound, Fans};
use super::motor::{Motor, MotorSound};
use super::tire::{Tire, TireSound};

/// Low enough to save work on the web, high enough for the hiss of water.
pub(super) const RATE: u32 = 32_000;
/// Samples between two reads of the [`Controls`].
pub(super) const BLOCK: usize = 32;
pub(super) const SOUND_SPEED: f32 = 343.0;
/// Seconds for the delay to catch up with where a source really is, on top
/// of following its speed.
const CATCH_UP: f32 = 0.3;
/// A source this many seconds of sound away from where it was heard has
/// jumped, and is heard from there at once.
const JUMP: f32 = 0.05;
/// Seconds for a voice to fade in and out.
const FADE: f32 = 0.15;
const GAIN_GLIDE: f32 = 0.02;
const STEREO: ChannelCount = ChannelCount::new(2).unwrap();

/// The sound of one vehicle, made as it plays from [`Controls`] the game
/// updates each frame. Each of its parts is heard from where it is.
#[derive(Asset, TypePath, Clone, Debug)]
pub(super) struct Voice {
    controls: Arc<Controls>,
    parts: Vec<Part>,
    /// Meters, for how far back the sound has to be kept.
    range: f32,
    seed: u32,
}

/// What makes the sound of a part of a vehicle.
#[derive(Clone, Copy, Debug)]
pub(super) enum Part {
    Tire(Tire),
    Motor(Motor),
    Fans(Fans),
}

/// A number shared with the audio thread.
#[derive(Debug, Default)]
pub(super) struct Shared(AtomicU32);

/// What the game tells a [`Voice`].
#[derive(Debug)]
pub(super) struct Controls {
    /// The mixer's gain times the volume of the voice.
    pub(super) gain: Shared,
    playing: AtomicBool,
    pub(super) parts: Box<[Heard]>,
}

/// How a part sounds and how the player hears it.
#[derive(Debug, Default)]
pub(super) struct Heard {
    /// In meters.
    pub(super) distance: Shared,
    /// Meters per second away from the player.
    pub(super) receding: Shared,
    pub(super) gains: [Shared; 2],
    /// Each ear hears little above this, in hertz.
    pub(super) cutoffs: [Shared; 2],
    /// Meters per second.
    pub(super) speed: Shared,
    /// Millimeters of water under a tire.
    pub(super) water: Shared,
}

/// A part's sound as it plays.
enum Model {
    Tire(Box<TireSound>),
    Motor(MotorSound),
    Fans(FanSound),
}

/// The way from a part to the ears: the sound arrives late by the time it
/// takes to travel, which shifts its pitch while the distance changes, and
/// each ear hears it at its own level and dullness.
struct Path {
    line: Vec<f32>,
    written: usize,
    /// In samples.
    delay: f32,
    /// Samples the delay grows by each sample in this block.
    slope: f32,
    started: bool,
    glides: [Glide; 2],
    gains: [f32; 2],
    steps: [f32; 2],
    pulls: [f32; 2],
    dull: [f32; 2],
}

/// Plays a [`Voice`] in stereo.
pub(super) struct Stream {
    controls: Arc<Controls>,
    parts: Vec<(Model, Path)>,
    fade: Glide,
    gain: f32,
    step: f32,
    left_in_block: usize,
    right: Option<f32>,
}

impl Voice {
    /// A voice of `parts`, heard up to `range` meters away, and its
    /// controls.
    pub(super) fn new(parts: Vec<Part>, range: f32, seed: u32) -> (Self, Arc<Controls>) {
        let controls = Arc::new(Controls {
            gain: Shared::default(),
            playing: AtomicBool::new(true),
            parts: parts.iter().map(|_| Heard::default()).collect(),
        });
        let voice = Self {
            controls: controls.clone(),
            parts,
            range,
            seed,
        };
        (voice, controls)
    }
}

impl Shared {
    pub(super) fn set(&self, value: f32) {
        self.0.store(value.to_bits(), Ordering::Relaxed);
    }

    pub(super) fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
}

impl Controls {
    /// Fades the voice out, after which it ends.
    pub(super) fn stop(&self) {
        self.playing.store(false, Ordering::Relaxed);
    }

    fn playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }
}

impl Model {
    fn new(part: Part, seed: u32) -> Self {
        match part {
            Part::Tire(tire) => Self::Tire(Box::new(tire.sound(seed))),
            Part::Motor(motor) => Self::Motor(motor.sound(seed)),
            Part::Fans(fans) => Self::Fans(fans.sound(seed)),
        }
    }

    fn block(&mut self, heard: &Heard) {
        let speed = heard.speed.get();
        match self {
            Self::Tire(tire) => tire.block(speed, heard.water.get()),
            Self::Motor(motor) => motor.block(speed),
            Self::Fans(fans) => fans.block(speed),
        }
    }

    fn next(&mut self) -> f32 {
        match self {
            Self::Tire(tire) => tire.next(),
            Self::Motor(motor) => motor.next(),
            Self::Fans(fans) => fans.next(),
        }
    }
}

impl Path {
    fn new(range: f32) -> Self {
        let length = (range / SOUND_SPEED * RATE as f32) as usize + 2 * BLOCK;
        Self {
            line: vec![0.0; length],
            written: 0,
            delay: 0.0,
            slope: 0.0,
            started: false,
            glides: [Glide::new(0.0, GAIN_GLIDE), Glide::new(0.0, GAIN_GLIDE)],
            gains: [0.0; 2],
            steps: [0.0; 2],
            pulls: [1.0; 2],
            dull: [0.0; 2],
        }
    }

    /// The delay grows as fast as the part moves away, which gives the
    /// Doppler shift `c / (c + v)` of a moving source, and catches up with
    /// the delay of where the part is.
    fn block(&mut self, heard: &Heard) {
        let receding = heard.receding.get();
        let rate = RATE as f32;
        let longest = (self.line.len() - BLOCK - 2) as f32;
        let target = (heard.distance.get() / (SOUND_SPEED + receding) * rate).clamp(1.0, longest);
        if !self.started || (target - self.delay).abs() > JUMP * rate {
            self.delay = target;
            self.started = true;
        }
        let catch_up = (target - self.delay) / (CATCH_UP * rate);
        self.slope = receding / (SOUND_SPEED + receding) + catch_up;
        for ear in 0..2 {
            let gain = self.glides[ear].follow(heard.gains[ear].get());
            self.steps[ear] = (gain - self.gains[ear]) / BLOCK as f32;
            self.pulls[ear] = one_pole(heard.cutoffs[ear].get());
        }
    }

    fn next(&mut self, sample: f32) -> [f32; 2] {
        let length = self.line.len();
        self.line[self.written] = sample;
        self.delay = (self.delay + self.slope).clamp(1.0, (length - 2) as f32);
        let back = self.written as f32 - self.delay;
        let back = if back < 0.0 {
            back + length as f32
        } else {
            back
        };
        let mix = back.fract();
        let index = back as usize % length;
        let heard = self.line[index] + (self.line[(index + 1) % length] - self.line[index]) * mix;
        self.written = (self.written + 1) % length;
        let mut out = [0.0; 2];
        for (ear, out) in out.iter_mut().enumerate() {
            self.gains[ear] += self.steps[ear];
            self.dull[ear] += (heard - self.dull[ear]) * self.pulls[ear];
            *out = self.dull[ear] * self.gains[ear];
        }
        out
    }
}

impl Stream {
    /// Reads the controls and sets up the next block.
    fn block(&mut self) -> bool {
        let playing = self.controls.playing();
        let target = if playing {
            self.controls.gain.get()
        } else {
            0.0
        };
        let gain = self.fade.follow(target);
        if !playing && gain < 1e-5 {
            return false;
        }
        self.step = (gain - self.gain) / BLOCK as f32;
        for ((model, path), heard) in self.parts.iter_mut().zip(&self.controls.parts) {
            model.block(heard);
            path.block(heard);
        }
        self.left_in_block = BLOCK;
        true
    }
}

impl Decodable for Voice {
    type Decoder = Stream;

    fn decoder(&self) -> Stream {
        let parts = self
            .parts
            .iter()
            .enumerate()
            .map(|(index, &part)| {
                let seed = self.seed.wrapping_mul(31).wrapping_add(index as u32);
                (Model::new(part, seed), Path::new(self.range))
            })
            .collect();
        Stream {
            controls: self.controls.clone(),
            parts,
            fade: Glide::new(0.0, FADE),
            gain: 0.0,
            step: 0.0,
            left_in_block: 0,
            right: None,
        }
    }
}

impl Iterator for Stream {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        if self.left_in_block == 0 && !self.block() {
            return None;
        }
        self.left_in_block -= 1;
        self.gain += self.step;
        let mut out = [0.0; 2];
        for (model, path) in &mut self.parts {
            let heard = path.next(model.next());
            out[0] += heard[0];
            out[1] += heard[1];
        }
        self.right = Some(out[1] * self.gain);
        Some(out[0] * self.gain)
    }
}

impl bevy::audio::Source for Stream {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        STEREO
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(RATE).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::*;

    /// Runs a 1 kHz tone through a path from a part `distance` meters away,
    /// moving away at `receding`, and returns the frequency and the peak
    /// heard over the last second of `seconds`.
    fn through_path(distance: f32, receding: f32, seconds: f32) -> (f32, f32) {
        let heard = Heard::default();
        heard.gains[0].set(1.0);
        heard.cutoffs[0].set(16_000.0);
        let mut path = Path::new(100.0);
        let count = (seconds * RATE as f32) as usize;
        let last = count - RATE as usize;
        let (mut crossings, mut previous, mut peak) = (0, 0.0, 0.0_f32);
        for i in 0..count {
            let t = i as f32 / RATE as f32;
            if i % BLOCK == 0 {
                heard.distance.set(distance + receding * t);
                heard.receding.set(receding);
                path.block(&heard);
            }
            let left = path.next((TAU * 1_000.0 * t).sin())[0];
            if i >= last {
                if previous < 0.0 && left >= 0.0 {
                    crossings += 1;
                }
                peak = peak.max(left.abs());
            }
            previous = left;
        }
        (crossings as f32, peak)
    }

    #[test]
    fn a_receding_part_sounds_lower() {
        let (hz, _) = through_path(10.0, 20.0, 3.0);
        let expected = 1_000.0 * SOUND_SPEED / (SOUND_SPEED + 20.0);
        assert!((hz - expected).abs() <= 2.0, "{hz} Hz, not {expected} Hz");
    }

    #[test]
    fn an_approaching_part_sounds_higher() {
        let (hz, _) = through_path(90.0, -20.0, 3.0);
        let expected = 1_000.0 * SOUND_SPEED / (SOUND_SPEED - 20.0);
        assert!((hz - expected).abs() <= 2.0, "{hz} Hz, not {expected} Hz");
    }

    #[test]
    fn the_delay_line_wraps_without_clicks() {
        // Long enough for the line to wrap many times.
        let (_, peak) = through_path(30.0, 15.0, 6.0);
        assert!(peak < 1.05, "{peak}");
    }

    #[test]
    fn a_stopped_voice_ends() {
        let fans = Fans {
            blades: 450.0,
            level: 1.0,
        };
        let (voice, controls) = Voice::new(vec![Part::Fans(fans)], 100.0, 1);
        controls.gain.set(1.0);
        let mut stream = voice.decoder();
        assert!(stream.by_ref().take(RATE as usize).count() == RATE as usize);
        controls.stop();
        let left = stream.take(10 * RATE as usize).count();
        assert!(left < 4 * RATE as usize, "{left}");
    }
}
