mod synth;

use std::sync::Arc;

use bevy::{audio::Volume, ecs::entity::EntityHashSet, platform::collections::HashMap, prelude::*};

use self::synth::{Noise, Samples};
use crate::parts::keypad::PressableKey;

/// How loud the game is while a menu is open.
const DUCKED: f32 = 0.35;
/// Keys this wide in meters play at their natural pitch. Wider keys sound
/// lower.
const KEY_WIDTH: f32 = 0.018;
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
    /// The rubber stamp hitting the ticket, or a device put down hard.
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
    /// A key switch bottoming out.
    KeyDown,
    /// A key switch springing back up.
    KeyUp,
}

impl Sound {
    /// The sounds played with [`Play`]. The radio plays its static itself,
    /// from where it stands.
    const ALL: [Self; 10] = [
        Self::Click,
        Self::Chime,
        Self::Thunk,
        Self::Ding,
        Self::Detent,
        Self::Ratchet,
        Self::Latch,
        Self::Unlatch,
        Self::KeyDown,
        Self::KeyUp,
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
            Self::KeyDown | Self::KeyUp => 2 * TAKES,
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
            Self::KeyDown => Samples::key_down(take),
            Self::KeyUp => Samples::key_up(take),
        }
    }

    pub(crate) fn volume(self) -> f32 {
        match self {
            Self::Click => 0.35,
            Self::Chime => 0.4,
            Self::Thunk => 0.6,
            Self::Ding => 0.45,
            Self::Detent => 0.5,
            Self::Ratchet => 0.12,
            Self::Latch | Self::Unlatch => 0.5,
            Self::Static => 0.4,
            Self::KeyDown | Self::KeyUp => 0.4,
        }
    }
}

/// How loud the game is overall.
#[derive(Resource, Debug)]
pub(crate) struct Mixer {
    /// From 0 to 1.
    pub(crate) volume: f32,
    /// Quieter while a menu is open.
    ducked: bool,
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

impl Default for Mixer {
    fn default() -> Self {
        Self {
            volume: 1.0,
            // The game starts on the title screen.
            ducked: true,
        }
    }
}

impl Mixer {
    /// What every sound is multiplied by.
    pub(crate) fn gain(&self) -> f32 {
        if self.ducked {
            self.volume * DUCKED
        } else {
            self.volume
        }
    }

    pub(crate) fn duck<const ON: bool>(mut mixer: ResMut<Mixer>) {
        mixer.ducked = ON;
    }
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyClicks>()
            .init_resource::<Mixer>()
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

    fn play(
        play: On<Play>,
        sounds: Option<ResMut<Sounds>>,
        mixer: Res<Mixer>,
        mut commands: Commands,
    ) {
        if let Some(mut sounds) = sounds
            && let Some(source) = sounds.0.get_mut(&play.0).map(|takes| takes.next().clone())
        {
            commands.spawn((
                AudioPlayer(source),
                PlaybackSettings::DESPAWN
                    .with_volume(Volume::Linear(play.0.volume() * mixer.gain())),
            ));
        }
    }

    /// Keys sound as they go down and as they come back up.
    fn click_keys(
        mut sounds: ResMut<Sounds>,
        enabled: Res<KeyClicks>,
        mixer: Res<Mixer>,
        keys: Query<(Entity, &PressableKey), Changed<PressableKey>>,
        mut down: Local<EntityHashSet>,
        mut commands: Commands,
    ) {
        for (entity, key) in &keys {
            let sound = if key.is_pressed() {
                if !down.insert(entity) {
                    continue;
                }
                Sound::KeyDown
            } else {
                if !down.remove(&entity) {
                    continue;
                }
                Sound::KeyUp
            };
            if !enabled.0 {
                continue;
            }
            let Some(takes) = sounds.0.get_mut(&sound) else {
                continue;
            };
            // Keys differ a little in pitch, so typing doesn't sound like a
            // loop.
            let own = 1.0 + (entity.to_bits() % 5) as f32 * 0.01 - 0.02;
            let pitch = (KEY_WIDTH / key.width()).powf(0.2) * own;
            commands.spawn((
                AudioPlayer(takes.next().clone()),
                PlaybackSettings::DESPAWN
                    .with_volume(Volume::Linear(sound.volume() * mixer.gain()))
                    .with_speed(pitch),
            ));
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
}
