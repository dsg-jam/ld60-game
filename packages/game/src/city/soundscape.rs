use bevy::{
    audio::{AudioSinkPlayback, SpatialAudioSink, SpatialScale, Volume},
    prelude::*,
};

use crate::audio::{Mixer, Sound};

/// So small that rodio's own fall off with the squared distance never kicks
/// in. [`Emitter`] fades with distance instead.
const SCALE: SpatialScale = SpatialScale::new(0.002);
/// In meters per second.
const SOUND_SPEED: f32 = 343.0;
/// Share of the way the heard speed of a source moves to its latest speed
/// each frame, so the Doppler shift doesn't jitter.
const DOPPLER_SMOOTHING: f32 = 0.1;
/// Seconds for an [`Emitter`] to follow a new level.
const GLIDE: f32 = 0.1;

pub(super) struct SoundscapePlugin;

impl Plugin for SoundscapePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Emitter::follow);
    }
}

/// A sound out in the city, heard from where it is. It fades with distance
/// and shifts in pitch while it moves towards or away from the player.
#[derive(Component, Debug)]
pub(super) struct Emitter {
    sound: Sound,
    /// Meters at which it is a quarter as loud as up close. It fades with
    /// the squared distance, faster than in open air, as the stalls and the
    /// crowd shield the street. Without one, it is as loud everywhere.
    reach: Option<f32>,
    pitch: f32,
    /// How loud it is right now, as a factor of its usual volume.
    pub(super) level: f32,
    /// [`Self::level`] as heard, gliding after it.
    heard: f32,
    distance: Option<f32>,
    /// Meters per second away from the player.
    receding: f32,
}

impl Emitter {
    pub(super) fn new(sound: Sound, reach: Option<f32>, pitch: f32) -> Self {
        Self {
            sound,
            reach,
            pitch,
            level: 0.0,
            heard: 0.0,
            distance: None,
            receding: 0.0,
        }
    }

    pub(super) fn with_level(mut self, level: f32) -> Self {
        self.level = level;
        self.heard = level;
        self
    }

    /// Plays `source` from here with `settings`, `distance` meters from the
    /// player.
    pub(super) fn bundle(
        self,
        source: Handle<AudioSource>,
        settings: PlaybackSettings,
        mixer: &Mixer,
        distance: f32,
    ) -> impl Bundle {
        (
            AudioPlayer(source),
            settings
                .with_spatial(true)
                .with_spatial_scale(SCALE)
                .with_speed(self.pitch)
                .with_volume(Volume::Linear(self.gain(mixer, distance))),
            self,
        )
    }

    fn gain(&self, mixer: &Mixer, distance: f32) -> f32 {
        let fade = self
            .reach
            .map_or(1.0, |reach| (reach / (reach + distance)).powi(2));
        self.sound.volume() * self.heard * fade * mixer.gain()
    }

    fn follow(
        time: Res<Time>,
        mixer: Res<Mixer>,
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        mut emitters: Query<(&mut Emitter, &GlobalTransform, &mut SpatialAudioSink)>,
    ) {
        let dt = time.delta_secs();
        let ear = listener.translation();
        for (mut emitter, transform, mut sink) in &mut emitters {
            let distance = transform.translation().distance(ear);
            if let Some(last) = emitter.distance
                && dt > 0.0
            {
                let receding = (distance - last) / dt;
                // Wrapping around the street is a jump, not a speed.
                if receding.abs() < SOUND_SPEED / 2.0 {
                    emitter.receding += (receding - emitter.receding) * DOPPLER_SMOOTHING;
                }
            }
            emitter.distance = Some(distance);
            emitter.heard += (emitter.level - emitter.heard) * (dt / GLIDE).min(1.0);
            sink.set_volume(Volume::Linear(emitter.gain(&mixer, distance)));
            sink.set_speed(emitter.pitch * SOUND_SPEED / (SOUND_SPEED + emitter.receding));
        }
    }
}
