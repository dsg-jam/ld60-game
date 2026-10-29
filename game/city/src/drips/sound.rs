use std::ops::Range;

use bevy::prelude::*;
use ld60_game_core::audio::{Mixer, Sound, Takes};
use rand::{RngExt, rngs::SmallRng};

use super::super::rng;
use super::super::soundscape::Emitter;
use super::super::street::water;
use super::super::weather::Weather;
use super::Drips;

const SEED: u64 = 13;
/// Only drips this close to the player are heard.
const RANGE: f32 = 9.0;
/// See [`Emitter`].
const REACH: f32 = 3.0;
/// Drops differ in size and in where they land.
const PITCH: Range<f32> = 0.85..1.15;
const LEVEL: Range<f32> = 0.3..1.0;

pub(super) struct DripSoundPlugin;

impl Plugin for DripSoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, DripSounds::load)
            .add_systems(Update, DripSounds::play.run_if(resource_exists::<Drips>));
    }
}

/// The sounds of drops landing, heard just as they land on screen.
#[derive(Resource)]
struct DripSounds {
    drip: Takes<Handle<AudioSource>>,
    plunk: Takes<Handle<AudioSource>>,
    rng: SmallRng,
    /// The time of the shaders when drops were last listened for.
    last: Option<f32>,
}

impl DripSounds {
    fn load(mut sources: ResMut<Assets<AudioSource>>, mut commands: Commands) {
        let mut takes = |sound: Sound, seed: u32| {
            let handles = sound
                .sources()
                .into_iter()
                .map(|source| sources.add(source))
                .collect();
            Takes::new(handles, seed)
        };
        commands.insert_resource(Self {
            drip: takes(Sound::Drip, 0),
            plunk: takes(Sound::Plunk, 1),
            rng: rng(SEED),
            last: None,
        });
    }

    /// Plays the drops that landed near the player since the last frame,
    /// with a splat on the ground or a plunk in a puddle.
    fn play(
        time: Res<Time>,
        weather: Res<Weather>,
        mixer: Res<Mixer>,
        drips: Res<Drips>,
        mut sounds: ResMut<DripSounds>,
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        mut commands: Commands,
    ) {
        let sounds = &mut *sounds;
        let now = time.elapsed_secs_wrapped();
        // The clock of the shaders wraps around now and then.
        let Some(last) = sounds.last.replace(now).filter(|&last| last < now) else {
            return;
        };
        let ear = listener.translation();
        for drip in &drips.0 {
            let ground = drip.anchor.with_y(0.0);
            let distance = ground.distance(ear);
            if distance > RANGE {
                continue;
            }
            for _ in drip.landing(last, now, weather.wetness) {
                let (sound, takes) = if water(ground.xz(), weather.wetness) > 0.0 {
                    (Sound::Plunk, &mut sounds.plunk)
                } else {
                    (Sound::Drip, &mut sounds.drip)
                };
                let emitter = Emitter::new(sound, Some(REACH), sounds.rng.random_range(PITCH))
                    .with_level(sounds.rng.random_range(LEVEL));
                commands.spawn((
                    Name::new("drip sound"),
                    emitter.bundle(
                        takes.next().clone(),
                        PlaybackSettings::DESPAWN,
                        &mixer,
                        distance,
                    ),
                    Transform::from_translation(ground),
                ));
            }
        }
    }
}
