mod synth;

use std::sync::Arc;

use bevy::{audio::Volume, ecs::entity::EntityHashSet, platform::collections::HashMap, prelude::*};

use self::synth::Samples;
use crate::parts::keypad::PressableKey;

const CLICK_VOLUME: f32 = 0.35;
/// Keys differ a little in pitch, so typing doesn't sound like a loop.
const CLICK_PITCHES: [f32; 5] = [0.92, 0.97, 1.0, 1.04, 1.08];

/// Sounds of interactions, all made in code.
pub(crate) struct AudioPlugin;

/// A one-shot sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Sound {
    /// A switch flipping.
    Click,
    /// A customer paid for a repair.
    Chime,
    /// The rubber stamp hitting the ticket.
    Thunk,
    /// The service bell for the customer.
    Ding,
}

impl Sound {
    const ALL: [Self; 4] = [Self::Click, Self::Chime, Self::Thunk, Self::Ding];

    fn samples(self) -> Samples {
        match self {
            Self::Click => Samples::click(),
            Self::Chime => Samples::chime(),
            Self::Thunk => Samples::thunk(),
            Self::Ding => Samples::ding(),
        }
    }

    fn volume(self) -> f32 {
        match self {
            Self::Click => CLICK_VOLUME,
            Self::Chime => 0.4,
            Self::Thunk => 0.6,
            Self::Ding => 0.45,
        }
    }
}

/// Plays a [`Sound`].
#[derive(Event, Debug)]
pub(crate) struct Play(pub(crate) Sound);

/// Whether pressing keys makes a sound.
#[derive(Resource, Debug)]
pub(crate) struct KeyClicks(pub(crate) bool);

impl Default for KeyClicks {
    fn default() -> Self {
        Self(true)
    }
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyClicks>()
            .add_systems(Startup, Sounds::load)
            .add_systems(Update, Sounds::click_keys.run_if(resource_exists::<Sounds>))
            .add_observer(Sounds::play);
    }
}

#[derive(Resource)]
struct Sounds(HashMap<Sound, Handle<AudioSource>>);

impl Sounds {
    fn load(mut sources: ResMut<Assets<AudioSource>>, mut commands: Commands) {
        let sounds = Sound::ALL
            .into_iter()
            .map(|sound| {
                let bytes = Arc::from(sound.samples().wav());
                (sound, sources.add(AudioSource { bytes }))
            })
            .collect();
        commands.insert_resource(Self(sounds));
    }

    fn play(play: On<Play>, sounds: Option<Res<Sounds>>, mut commands: Commands) {
        if let Some(source) = sounds.and_then(|sounds| sounds.0.get(&play.0).cloned()) {
            commands.spawn((
                AudioPlayer(source),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(play.0.volume())),
            ));
        }
    }

    fn click_keys(
        sounds: Res<Sounds>,
        enabled: Res<KeyClicks>,
        keys: Query<(Entity, &PressableKey), Changed<PressableKey>>,
        mut down: Local<EntityHashSet>,
        mut commands: Commands,
    ) {
        for (entity, key) in &keys {
            if !key.is_pressed() {
                down.remove(&entity);
            } else if down.insert(entity)
                && enabled.0
                && let Some(click) = sounds.0.get(&Sound::Click)
            {
                let pitch = CLICK_PITCHES[entity.to_bits() as usize % CLICK_PITCHES.len()];
                commands.spawn((
                    AudioPlayer(click.clone()),
                    PlaybackSettings::DESPAWN
                        .with_volume(Volume::Linear(CLICK_VOLUME))
                        .with_speed(pitch),
                ));
            }
        }
    }
}
