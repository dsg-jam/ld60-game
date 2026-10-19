mod pan;

use std::error::Error;

use bevy::{asset::LoadState, audio::Volume, prelude::*};

use self::pan::{Pan, Panned};
use super::{Front, Radio};
use crate::audio::{Mixer, Sound, Takes};

/// Station n starts with track n and plays on through the ones after it.
pub(super) const TRACKS: [Track; 11] = [
    Track::new("stargazer-01-synthetic", ESCP, "Synthetic"),
    Track::new("nostalgia", "GLORYTOTHEMACHINE", "NOSTALGIA"),
    Track::new("stargazer-02-open-space", ESCP, "Open Space"),
    Track::new("stargazer-03-flying-home", ESCP, "Flying Home"),
    Track::new("exploration-theme", "Cleyton Kauffman", "Exploration Theme"),
    Track::new("stargazer-04-sunset", ESCP, "Sunset"),
    Track::new("popi", ZENTAZY, "Popi"),
    Track::new("stargazer-05-stargazer", ESCP, "Stargazer"),
    Track::new("stargazer-06-above-the-clouds", ESCP, "Above The Clouds"),
    Track::new("bobi-bob", ZENTAZY, "Bobi Bob"),
    Track::new("stargazer-07-in-love", ESCP, "In Love"),
];
const ESCP: &str = "e s c p";
const ZENTAZY: &str = "Zentazy";
/// Seconds between the end of one track and the start of the next.
const PAUSE: f32 = 2.5;

/// A piece of music in `assets/music`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Track {
    file: &'static str,
    pub(super) artist: &'static str,
    pub(super) title: &'static str,
}

pub(super) struct BroadcastPlugin;

impl Plugin for BroadcastPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(pan::PanPlugin)
            .insert_resource(StaticTakes(Takes::new(Sound::Static.sources(), 0)))
            .add_systems(
                Update,
                (Tuner::tune, Broadcast::start, Broadcast::set_volume).chain(),
            );
    }
}

/// What a [`Radio`] plays, and what it plays next.
#[derive(Component, Debug, Default)]
pub(super) enum Tuner {
    /// Nothing has played yet.
    #[default]
    Idle,
    /// `track` plays for the station the dial is on.
    Playing { station: usize, track: usize },
    /// `track` comes on once the pause after the one before is over.
    Pause {
        station: usize,
        track: usize,
        until: f32,
    },
}

/// The music coming out of a [`Radio`]'s speaker. It starts once the
/// track has loaded.
#[derive(Component, Debug)]
#[require(Pan)]
struct Broadcast {
    music: Handle<AudioSource>,
}

/// The sound of a radio between two stations, while tuning.
#[derive(Component, Debug)]
#[require(Pan)]
struct Static;

/// The takes of [`Static`].
#[derive(Resource)]
struct StaticTakes(Takes<AudioSource>);

impl Tuner {
    /// The track playing or about to play, if any.
    pub(super) fn track(&self) -> Option<usize> {
        match *self {
            Self::Idle => None,
            Self::Playing { track, .. } | Self::Pause { track, .. } => Some(track),
        }
    }

    fn station(&self) -> Option<usize> {
        match *self {
            Self::Idle => None,
            Self::Playing { station, .. } | Self::Pause { station, .. } => Some(station),
        }
    }

    /// Starts the first track, pauses and moves on when one ends, and
    /// switches over with some static when the player tunes in to another
    /// station.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn tune(
        mut commands: Commands,
        time: Res<Time>,
        mixer: Res<Mixer>,
        assets: Res<AssetServer>,
        mut statics: ResMut<StaticTakes>,
        mut panned: ResMut<Assets<Panned>>,
        mut radios: Query<(Entity, &Radio, &mut Tuner, Option<&Children>)>,
        broadcasts: Query<(), With<Broadcast>>,
        playing_static: Query<(), With<Static>>,
    ) {
        let now = time.elapsed_secs();
        for (entity, radio, mut tuner, children) in &mut radios {
            let playing = children
                .into_iter()
                .flatten()
                .copied()
                .find(|&child| broadcasts.contains(child));
            let track = if tuner
                .station()
                .is_some_and(|station| station != radio.station)
            {
                // Quick turns of the knob don't pile up the static.
                let old = children.into_iter().flatten().copied();
                for child in old
                    .filter(|&child| playing_static.contains(child))
                    .chain(playing)
                {
                    commands.entity(child).despawn();
                }
                if radio.on {
                    let pan = Pan::default();
                    commands.entity(entity).with_child((
                        Static,
                        AudioPlayer(panned.add(Panned::new(statics.0.next().clone(), pan.clone()))),
                        PlaybackSettings::DESPAWN
                            .with_volume(Volume::Linear(Static::gain(radio) * mixer.gain())),
                        pan,
                        Radio::speaker(),
                    ));
                }
                radio.station
            } else {
                match *tuner {
                    Self::Idle => radio.station,
                    Self::Playing { track, .. } => {
                        if playing.is_none() {
                            *tuner = Self::Pause {
                                station: radio.station,
                                track: (track + 1) % TRACKS.len(),
                                until: now + PAUSE,
                            };
                        }
                        continue;
                    }
                    Self::Pause { track, until, .. } => {
                        if now < until {
                            continue;
                        }
                        track
                    }
                }
            };
            *tuner = Self::Playing {
                station: radio.station,
                track,
            };
            commands.entity(entity).with_child((
                Broadcast {
                    music: assets.load(TRACKS[track].path()),
                },
                Radio::speaker(),
            ));
        }
    }
}

impl Broadcast {
    /// Plays the music once it has loaded. Music that doesn't load ends
    /// right away.
    fn start(
        mut commands: Commands,
        assets: Res<AssetServer>,
        mixer: Res<Mixer>,
        sources: Res<Assets<AudioSource>>,
        mut panned: ResMut<Assets<Panned>>,
        broadcasts: Query<(Entity, &Broadcast, &Pan, &ChildOf), Without<AudioPlayer<Panned>>>,
        radios: Query<&Radio>,
    ) {
        for (entity, broadcast, pan, child_of) in &broadcasts {
            if let Some(LoadState::Failed(err)) = assets.get_load_state(&broadcast.music) {
                error!(error = &*err as &dyn Error, "radio music does not load");
                commands.entity(entity).despawn();
                continue;
            }
            let (Some(music), Ok(radio)) =
                (sources.get(&broadcast.music), radios.get(child_of.parent()))
            else {
                continue;
            };
            commands.entity(entity).insert((
                AudioPlayer(panned.add(Panned::new(music.clone(), pan.clone()))),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(radio.gain() * mixer.gain())),
            ));
        }
    }

    /// Follows the volume knob and the power switch.
    fn set_volume(
        mixer: Res<Mixer>,
        radios: Query<(&Radio, &Children)>,
        mut music: Query<&mut AudioSink, (With<Broadcast>, Without<Static>)>,
        mut static_sinks: Query<&mut AudioSink, (With<Static>, Without<Broadcast>)>,
    ) {
        for (radio, children) in &radios {
            for &child in children {
                if let Ok(mut sink) = music.get_mut(child) {
                    sink.set_volume(Volume::Linear(radio.gain() * mixer.gain()));
                }
                if let Ok(mut sink) = static_sinks.get_mut(child) {
                    sink.set_volume(Volume::Linear(Static::gain(radio) * mixer.gain()));
                }
            }
        }
    }
}

impl Static {
    fn gain(radio: &Radio) -> f32 {
        radio.gain() * Sound::Static.volume()
    }
}

impl Track {
    const fn new(file: &'static str, artist: &'static str, title: &'static str) -> Self {
        Self {
            file,
            artist,
            title,
        }
    }

    fn path(&self) -> String {
        format!("music/{}.ogg", self.file)
    }
}

impl Radio {
    /// Where the sound comes from: behind the grille.
    fn speaker() -> Transform {
        Transform::from_translation(Front::grille().center)
    }

    /// Squared, so the knob feels even.
    fn gain(&self) -> f32 {
        if self.on {
            self.volume * self.volume
        } else {
            0.0
        }
    }
}
