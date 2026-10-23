use std::error::Error;
use std::f32::consts::{FRAC_PI_4, TAU};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use bevy::{
    audio::{AddAudioSource, ChannelCount, Decodable, Sample, SampleRate, Source},
    prelude::*,
};

use ld60_core::player::camera::PlayerCamera;

/// How far a sound right at the side moves over, from 0 for the middle to 1
/// for only one ear. Real heads let some sound round to the far ear.
const WIDTH: f32 = 0.8;
/// How much duller the far ear hears a sound right at the side.
const SHADOW: f32 = 0.7;
/// Sounds from behind are a little duller in both ears.
const BEHIND: f32 = 0.3;
/// How long gains take to follow a turn of the head, so changes don't
/// click.
const GLIDE_SECONDS: f32 = 0.015;
/// The shadowed ear hears little above this frequency, in hertz.
const MUFFLE_HZ: f32 = 2_000.0;
const STEREO: ChannelCount = ChannelCount::new(2).unwrap();

pub(super) struct PanPlugin;

impl Plugin for PanPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<Panned>()
            .add_systems(PostUpdate, Pan::follow.after(TransformSystems::Propagate))
            .add_observer(Listener::attach);
    }
}

/// A sound that plays louder in the ear facing it, and duller in the other.
#[derive(Asset, TypePath, Clone, Debug)]
pub(super) struct Panned {
    source: AudioSource,
    pan: Pan,
    /// Where in the source it starts.
    start: Duration,
}

/// What each ear hears of a [`Panned`] sound, on the entity it comes from.
/// Shared with the audio thread.
#[derive(Component, Clone, Debug, Default)]
pub(super) struct Pan(Arc<Ears>);

/// Gain and dullness of the left and right ear, as `f32` bits.
#[derive(Debug, Default)]
struct Ears {
    gains: [AtomicU32; 2],
    dull: [AtomicU32; 2],
}

/// Where the player hears sounds from.
#[derive(Component, Debug)]
struct Listener;

impl Panned {
    /// `source` coming from where the entity with `pan` is.
    pub(super) fn new(source: AudioSource, pan: Pan) -> Self {
        Self {
            source,
            pan,
            start: Duration::ZERO,
        }
    }

    /// Starts `start` into the source.
    pub(super) fn from(self, start: Duration) -> Self {
        Self { start, ..self }
    }
}

impl Pan {
    fn set(&self, gains: [f32; 2], dull: [f32; 2]) {
        for ear in 0..2 {
            self.0.gains[ear].store(gains[ear].to_bits(), Ordering::Relaxed);
            self.0.dull[ear].store(dull[ear].to_bits(), Ordering::Relaxed);
        }
    }

    fn gain(&self, ear: usize) -> f32 {
        f32::from_bits(self.0.gains[ear].load(Ordering::Relaxed))
    }

    fn dull(&self, ear: usize) -> f32 {
        f32::from_bits(self.0.dull[ear].load(Ordering::Relaxed))
    }

    /// Gains and dullness of the left and right ear for a sound in
    /// `direction`, in the listener's space.
    fn hear(direction: Vec3) -> ([f32; 2], [f32; 2]) {
        let side = (direction.x * WIDTH).clamp(-1.0, 1.0);
        // Equal power, full gain in the near ear at the side.
        let angle = (side + 1.0) * FRAC_PI_4;
        let gains = [angle.cos(), angle.sin()];
        let behind = direction.z.max(0.0) * BEHIND;
        let dull = [
            (side.max(0.0) * SHADOW + behind).min(1.0),
            ((-side).max(0.0) * SHADOW + behind).min(1.0),
        ];
        (gains, dull)
    }

    fn follow(
        listener: Single<&GlobalTransform, With<Listener>>,
        sounds: Query<(&Pan, &GlobalTransform)>,
    ) {
        let into_view = listener.affine().inverse();
        for (pan, transform) in &sounds {
            let direction = into_view
                .transform_point3(transform.translation())
                .normalize_or(Vec3::NEG_Z);
            let (gains, dull) = Self::hear(direction);
            pan.set(gains, dull);
        }
    }
}

impl Listener {
    /// The player hears from where the camera is.
    fn attach(add: On<Add<PlayerCamera>>, mut commands: Commands) {
        commands.entity(add.entity).insert(Listener);
    }
}

impl Decodable for Panned {
    type Decoder = Stereo;

    fn decoder(&self) -> Self::Decoder {
        let mut inner = self.source.decoder();
        if !self.start.is_zero()
            && let Err(err) = inner.try_seek(self.start)
        {
            warn!(error = &err as &dyn Error, "radio music does not seek");
        }
        let rate = inner.sample_rate().get() as f32;
        let pan = &self.pan;
        Stereo {
            inner,
            pan: pan.clone(),
            gains: [pan.gain(0), pan.gain(1)],
            dull: [pan.dull(0), pan.dull(1)],
            glide: 1.0 - (-1.0 / (rate * GLIDE_SECONDS)).exp(),
            muffle: 1.0 - (-TAU * MUFFLE_HZ / rate).exp(),
            muffled: [0.0; 2],
            right: None,
        }
    }
}

/// Plays a sound in two channels, one for each ear.
pub(super) struct Stereo {
    inner: <AudioSource as Decodable>::Decoder,
    pan: Pan,
    gains: [f32; 2],
    dull: [f32; 2],
    /// Share of the way to the new gains each sample.
    glide: f32,
    /// One pole low-pass filter of the shadowed ear.
    muffle: f32,
    /// Low passed sound in each ear.
    muffled: [f32; 2],
    /// The right channel of the current frame, once the left one is out.
    right: Option<Sample>,
}

impl Stereo {
    /// The next sample in mono, mixed down from all channels.
    fn next_mono(&mut self) -> Option<f32> {
        let channels = self.inner.channels().get();
        let mut sum = 0.0;
        for _ in 0..channels {
            sum += self.inner.next()?;
        }
        Some(sum / f32::from(channels))
    }
}

impl Iterator for Stereo {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        let mono = self.next_mono()?;
        let mut out = [0.0; 2];
        for (ear, sample) in out.iter_mut().enumerate() {
            self.gains[ear] += (self.pan.gain(ear) - self.gains[ear]) * self.glide;
            self.dull[ear] += (self.pan.dull(ear) - self.dull[ear]) * self.glide;
            self.muffled[ear] += (mono - self.muffled[ear]) * self.muffle;
            let heard = mono + (self.muffled[ear] - mono) * self.dull[ear];
            *sample = heard * self.gains[ear];
        }
        self.right = Some(out[1]);
        Some(out[0])
    }
}

impl Source for Stereo {
    fn current_span_len(&self) -> Option<usize> {
        let channels = usize::from(self.inner.channels().get());
        self.inner
            .current_span_len()
            .map(|len| len / channels * 2 + usize::from(self.right.is_some()))
    }

    fn channels(&self) -> ChannelCount {
        STEREO
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_in_front_are_even() {
        let (gains, dull) = Pan::hear(Vec3::NEG_Z);
        assert!((gains[0] - gains[1]).abs() < 1e-5);
        assert_eq!(dull, [0.0, 0.0]);
    }

    #[test]
    fn sounds_on_the_right_favor_the_right_ear() {
        let (gains, dull) = Pan::hear(Vec3::X);
        // Over 15 dB quieter on the left, and never louder than 1.
        assert!(gains[0] / gains[1] < 0.18);
        assert!(gains[1] <= 1.0);
        assert!(dull[0] > dull[1]);
    }
}
