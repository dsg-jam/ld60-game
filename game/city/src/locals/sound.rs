use std::ops::Range;
use std::time::Duration;

use bevy::prelude::*;
use ld60_game_audio::{Mixer, Sound, Takes};
use rand::RngExt;
use rand::rngs::SmallRng;

use super::super::rng;
use super::super::soundscape::Emitter;
use super::super::street::water;
use super::super::weather::Weather;
use super::umbrella::Canopy;

const SEED: u64 = 17;
/// Only feet and voices this close to the player are heard.
const STEP_RANGE: f32 = 14.0;
const VOICE_RANGE: f32 = 18.0;
/// See [`Emitter`].
const STEP_REACH: f32 = 3.0;
const VOICE_REACH: f32 = 4.0;
/// The ground counts as wet from here on.
const WET: f32 = 0.2;
const STEP_PITCH: Range<f32> = 0.9..1.1;
/// How much each phrase differs in pitch from the voice.
const INTONATION: Range<f32> = 0.94..1.06;
/// Height of the mouth above the feet, before size.
const MOUTH: f32 = 1.6;
/// Rain on umbrellas is heard this close, and stops beyond the other.
const PATTER_RANGE: f32 = 9.0;
const PATTER_GONE: f32 = 10.0;
const PATTER_REACH: f32 = 2.0;
const PATTER_PITCH: Range<f32> = 0.92..1.08;

pub(super) struct LocalSoundPlugin;

impl Plugin for LocalSoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Footfalls>()
            .add_systems(Startup, LocalSounds::load)
            .add_systems(
                Update,
                (LocalSounds::step, LocalSounds::talk, LocalSounds::patter),
            );
    }
}

/// Feet that landed this frame.
#[derive(Resource, Debug, Default)]
pub(super) struct Footfalls(pub(super) Vec<Footfall>);

#[derive(Debug, Clone, Copy)]
pub(super) struct Footfall {
    pub(super) at: Vec3,
    /// How hard, from 0 to 1.
    pub(super) level: f32,
    /// Whether a roof keeps the ground dry.
    pub(super) indoors: bool,
}

/// A local's voice, and when it next says something.
#[derive(Component, Debug)]
pub(super) struct Voice {
    pitch: f32,
    /// Seconds between phrases.
    every: Range<f32>,
    until: f32,
}

impl Voice {
    /// A voice for a body of `size`, lower for larger ones.
    pub(super) fn new(size: f32, every: Range<f32>, rng: &mut impl rand::Rng) -> Self {
        Self {
            pitch: rng.random_range(0.9..1.1) / size,
            until: rng.random_range(every.clone()),
            every,
        }
    }
}

#[derive(Resource)]
struct LocalSounds {
    dry: Takes<Handle<AudioSource>>,
    wet: Takes<Handle<AudioSource>>,
    splash: Takes<Handle<AudioSource>>,
    murmur: Takes<Handle<AudioSource>>,
    patter: Takes<Handle<AudioSource>>,
    rng: SmallRng,
}

impl LocalSounds {
    fn load(mut sources: ResMut<Assets<AudioSource>>, mut commands: Commands) {
        let mut handles = |sound: Sound| {
            sound
                .sources()
                .into_iter()
                .map(|source| sources.add(source))
                .collect::<Vec<_>>()
        };
        let (wet, dry): (Vec<_>, Vec<_>) = handles(Sound::Footstep)
            .into_iter()
            .enumerate()
            .partition(|(take, _)| take % 2 == 1);
        let unzip = |takes: Vec<(usize, Handle<AudioSource>)>| {
            takes.into_iter().map(|(_, handle)| handle).collect()
        };
        commands.insert_resource(Self {
            dry: Takes::new(unzip(dry), 0),
            wet: Takes::new(unzip(wet), 1),
            splash: Takes::new(handles(Sound::Splash), 2),
            murmur: Takes::new(handles(Sound::Murmur), 3),
            patter: Takes::new(handles(Sound::UmbrellaRain), 4),
            rng: rng(SEED),
        });
    }

    /// Plays the feet that landed near the player: a tap, wetter on a wet
    /// sidewalk, or a splash in a puddle.
    fn step(
        weather: Res<Weather>,
        mixer: Res<Mixer>,
        mut footfalls: ResMut<Footfalls>,
        sounds: Option<ResMut<LocalSounds>>,
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        mut commands: Commands,
    ) {
        let Some(mut sounds) = sounds else {
            footfalls.0.clear();
            return;
        };
        let sounds = &mut *sounds;
        let ear = listener.translation();
        for Footfall { at, level, indoors } in footfalls.0.drain(..) {
            let distance = at.distance(ear);
            if distance > STEP_RANGE {
                continue;
            }
            let (sound, takes) = if indoors {
                (Sound::Footstep, &mut sounds.dry)
            } else if water(at.xz(), weather.wetness) > 0.0 {
                (Sound::Splash, &mut sounds.splash)
            } else if weather.wetness > WET {
                (Sound::Footstep, &mut sounds.wet)
            } else {
                (Sound::Footstep, &mut sounds.dry)
            };
            let emitter =
                Emitter::new(sound, Some(STEP_REACH), sounds.rng.random_range(STEP_PITCH))
                    .with_level(level);
            commands.spawn((
                Name::new("footstep sound"),
                emitter.bundle(
                    takes.next().clone(),
                    PlaybackSettings::DESPAWN,
                    &mixer,
                    distance,
                ),
                Transform::from_translation(at),
            ));
        }
    }

    /// Now and then, a local near the player says something.
    fn talk(
        time: Res<Time>,
        mixer: Res<Mixer>,
        sounds: Option<ResMut<LocalSounds>>,
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        mut voices: Query<(&mut Voice, &GlobalTransform)>,
        mut commands: Commands,
    ) {
        let Some(mut sounds) = sounds else {
            return;
        };
        let sounds = &mut *sounds;
        let dt = time.delta_secs();
        let ear = listener.translation();
        for (mut voice, body) in &mut voices {
            voice.until -= dt;
            if voice.until > 0.0 {
                continue;
            }
            voice.until = sounds.rng.random_range(voice.every.clone());
            let mouth = body.transform_point(Vec3::Y * MOUTH);
            let distance = mouth.distance(ear);
            if distance > VOICE_RANGE {
                continue;
            }
            let pitch = voice.pitch * sounds.rng.random_range(INTONATION);
            let emitter = Emitter::new(Sound::Murmur, Some(VOICE_REACH), pitch).with_level(1.0);
            commands.spawn((
                Name::new("voice"),
                emitter.bundle(
                    sounds.murmur.next().clone(),
                    PlaybackSettings::DESPAWN,
                    &mixer,
                    distance,
                ),
                Transform::from_translation(mouth),
            ));
        }
    }

    /// Rain drums on the open umbrellas near the player.
    fn patter(
        weather: Res<Weather>,
        mixer: Res<Mixer>,
        sounds: Option<ResMut<LocalSounds>>,
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        mut canopies: Query<(Entity, &mut Canopy, &GlobalTransform)>,
        mut emitters: Query<&mut Emitter>,
        mut commands: Commands,
    ) {
        let Some(mut sounds) = sounds else {
            return;
        };
        let sounds = &mut *sounds;
        let ear = listener.translation();
        for (entity, mut canopy, at) in &mut canopies {
            let distance = at.translation().distance(ear);
            let level = weather.rain * canopy.open;
            match canopy.patter {
                Some(patter) if level <= 0.0 || distance > PATTER_GONE => {
                    commands.entity(patter).despawn();
                    canopy.patter = None;
                }
                Some(patter) => {
                    if let Ok(mut emitter) = emitters.get_mut(patter) {
                        emitter.level = level;
                    }
                }
                None if level > 0.0 && distance < PATTER_RANGE => {
                    let pitch = sounds.rng.random_range(PATTER_PITCH);
                    let start = Duration::from_secs_f32(sounds.rng.random_range(0.0..4.0));
                    let emitter = Emitter::new(Sound::UmbrellaRain, Some(PATTER_REACH), pitch)
                        .with_level(level);
                    let patter = commands
                        .spawn((
                            Name::new("rain on an umbrella"),
                            emitter.bundle(
                                sounds.patter.next().clone(),
                                PlaybackSettings::LOOP.with_start_position(start),
                                &mixer,
                                distance,
                            ),
                            Transform::default(),
                            ChildOf(entity),
                        ))
                        .id();
                    canopy.patter = Some(patter);
                }
                None => {}
            }
        }
    }
}
