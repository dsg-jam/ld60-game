mod synth;

use std::sync::Arc;

use bevy::{audio::Volume, ecs::entity::EntityHashSet, platform::collections::HashMap, prelude::*};

use self::synth::{Noise, Samples};
use crate::parts::keypad::PressableKey;

const CLICK_VOLUME: f32 = 0.35;
/// Keys differ a little in pitch, so typing doesn't sound like a loop.
const CLICK_PITCHES: [f32; 5] = [0.92, 0.97, 1.0, 1.04, 1.08];
/// Sounds that would stand out if they always came the same get this many
/// takes, played in random order.
const TAKES: u32 = 6;

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
    /// A knob snapping into its next position.
    Detent,
    /// A smooth knob's soft ratchet, ticking as it turns.
    Ratchet,
    /// A push button latching down.
    Latch,
    /// A push button springing back up.
    Unlatch,
    /// A radio between two stations.
    Static,
}

impl Sound {
    /// The sounds played with [`Play`]. The radio plays its static itself,
    /// from where it stands.
    const ALL: [Self; 8] = [
        Self::Click,
        Self::Chime,
        Self::Thunk,
        Self::Ding,
        Self::Detent,
        Self::Ratchet,
        Self::Latch,
        Self::Unlatch,
    ];

    /// The takes of the sound as assets, for playing them in other ways
    /// than [`Play`].
    pub(crate) fn sources(self) -> Vec<AudioSource> {
        (0..self.takes())
            .map(|take| AudioSource {
                bytes: Arc::from(self.samples(take).wav()),
            })
            .collect()
    }

    fn takes(self) -> u32 {
        match self {
            Self::Detent | Self::Ratchet | Self::Latch | Self::Unlatch | Self::Static => TAKES,
            Self::Click | Self::Chime | Self::Thunk | Self::Ding => 1,
        }
    }

    fn samples(self, take: u32) -> Samples {
        match self {
            Self::Click => Samples::click(),
            Self::Chime => Samples::chime(),
            Self::Thunk => Samples::thunk(),
            Self::Ding => Samples::ding(),
            Self::Detent => Samples::detent(take),
            Self::Ratchet => Samples::ratchet(take),
            Self::Latch => Samples::latch(take),
            Self::Unlatch => Samples::unlatch(take),
            Self::Static => Samples::radio_static(take),
        }
    }

    pub(crate) fn volume(self) -> f32 {
        match self {
            Self::Click => CLICK_VOLUME,
            Self::Chime => 0.4,
            Self::Thunk => 0.6,
            Self::Ding => 0.45,
            Self::Detent => 0.5,
            Self::Ratchet => 0.12,
            Self::Latch | Self::Unlatch => 0.5,
            Self::Static => 0.4,
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
struct Sounds(HashMap<Sound, Takes<Handle<AudioSource>>>);

/// The takes of a sound, picked at random.
pub(crate) struct Takes<T> {
    takes: Vec<T>,
    dice: Noise,
    /// The take picked last.
    last: Option<usize>,
}

impl Sounds {
    fn load(mut sources: ResMut<Assets<AudioSource>>, mut commands: Commands) {
        let sounds = Sound::ALL
            .into_iter()
            .enumerate()
            .map(|(index, sound)| {
                let handles = sound
                    .sources()
                    .into_iter()
                    .map(|source| sources.add(source));
                (sound, Takes::new(handles.collect(), index as u32))
            })
            .collect();
        commands.insert_resource(Self(sounds));
    }

    fn play(play: On<Play>, sounds: Option<ResMut<Sounds>>, mut commands: Commands) {
        if let Some(mut sounds) = sounds
            && let Some(source) = sounds.0.get_mut(&play.0).map(|takes| takes.next().clone())
        {
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
                && let Some(click) = sounds.0.get(&Sound::Click).and_then(Takes::first)
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

impl<T> Takes<T> {
    /// Picks among `takes` in an order made from `seed`.
    pub(crate) fn new(takes: Vec<T>, seed: u32) -> Self {
        Self {
            takes,
            dice: Noise::new(seed),
            last: None,
        }
    }

    /// One of the takes, never the same one twice in a row unless there is
    /// only one.
    pub(crate) fn next(&mut self) -> &T {
        let count = self.takes.len();
        let roll = (self.dice.next() + 1.0) / 2.0;
        let mut pick = ((roll * count as f32) as usize).min(count.saturating_sub(1));
        if count > 1 && Some(pick) == self.last {
            pick = (pick + 1) % count;
        }
        self.last = Some(pick);
        &self.takes[pick]
    }

    fn first(&self) -> Option<&T> {
        self.takes.first()
    }
}
