mod pan;
mod program;

use std::error::Error;
use std::time::Duration;

use bevy::{asset::LoadState, audio::Volume, prelude::*};
use ld60_game_core::audio::{Mixer, Sound, Takes};
use ld60_game_core::state::AppState;

use self::pan::{Pan, Panned};
use self::program::OnAir;
use super::{Front, Radio};

/// Station n starts with track n and plays on through the ones after it.
pub(super) const TRACKS: [Track; 11] = [
    Track::new("stargazer-01-synthetic", ESCP, "Synthetic", 240.6),
    Track::new("nostalgia", "GLORYTOTHEMACHINE", "NOSTALGIA", 183.0),
    Track::new("stargazer-02-open-space", ESCP, "Open Space", 238.6),
    Track::new("stargazer-03-flying-home", ESCP, "Flying Home", 273.6),
    Track::new(
        "exploration-theme",
        "Cleyton Kauffman",
        "Exploration Theme",
        134.4,
    ),
    Track::new("stargazer-04-sunset", ESCP, "Sunset", 270.1),
    Track::new("popi", ZENTAZY, "Popi", 214.0),
    Track::new("stargazer-05-stargazer", ESCP, "Stargazer", 343.3),
    Track::new(
        "stargazer-06-above-the-clouds",
        ESCP,
        "Above The Clouds",
        373.0,
    ),
    Track::new("bobi-bob", ZENTAZY, "Bobi Bob", 152.1),
    Track::new("stargazer-07-in-love", ESCP, "In Love", 275.0),
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
    /// In seconds.
    length: f32,
}

pub(super) struct BroadcastPlugin;

impl Plugin for BroadcastPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(pan::PanPlugin)
            .insert_resource(StaticTakes(Takes::new(Sound::Static.sources(), 0)))
            .add_systems(
                Update,
                (Tuner::tune, Broadcast::start, Broadcast::set_volume)
                    .chain()
                    .run_if(not(in_state(AppState::Loading))),
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
        /// On the broadcast clock.
        until: f64,
    },
}

/// The music coming out of a [`Radio`]'s speaker. It starts once the
/// track has loaded, at the point the station has reached by then.
#[derive(Component, Debug)]
#[require(Pan)]
struct Broadcast {
    music: Handle<AudioSource>,
    station: usize,
    track: usize,
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

    /// Tunes in to what the station has on air, and follows its program
    /// when a track ends. A turn to another station comes with some
    /// static.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn tune(
        mut commands: Commands,
        time: Res<Time<Real>>,
        mixer: Res<Mixer>,
        assets: Res<AssetServer>,
        mut statics: ResMut<StaticTakes>,
        mut panned: ResMut<Assets<Panned>>,
        mut radios: Query<(Entity, &Radio, &mut Tuner, Option<&Children>)>,
        broadcasts: Query<(), With<Broadcast>>,
        playing_static: Query<(), With<Static>>,
    ) {
        let clock = time.elapsed_secs_f64();
        for (entity, radio, mut tuner, children) in &mut radios {
            let station = radio.station;
            let playing = children
                .into_iter()
                .flatten()
                .copied()
                .find(|&child| broadcasts.contains(child));
            let next = match *tuner {
                Self::Idle => Self::on_air(station, clock),
                Self::Playing { station: was, .. } | Self::Pause { station: was, .. }
                    if was != station =>
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
                            AudioPlayer(
                                panned.add(Panned::new(statics.0.next().clone(), pan.clone())),
                            ),
                            PlaybackSettings::DESPAWN
                                .with_volume(Volume::Linear(Static::gain(radio) * mixer.gain())),
                            pan,
                            Radio::speaker(),
                        ));
                    }
                    Self::on_air(station, clock)
                }
                Self::Playing { track, .. } => {
                    if playing.is_some() {
                        continue;
                    }
                    match OnAir::at(station, clock) {
                        // Ended a little early, so the rest of its slot is
                        // part of the pause.
                        OnAir::Track { track: on, at } if on == track => Self::Pause {
                            station,
                            track: (track + 1) % TRACKS.len(),
                            until: clock + f64::from(TRACKS[track].length - at + PAUSE),
                        },
                        _ => Self::on_air(station, clock),
                    }
                }
                Self::Pause { track, until, .. } => {
                    if clock < until {
                        continue;
                    }
                    Self::Playing { station, track }
                }
            };
            if let Self::Playing { track, .. } = next {
                commands.entity(entity).with_child((
                    Broadcast {
                        music: assets.load(TRACKS[track].path()),
                        station,
                        track,
                    },
                    Radio::speaker(),
                ));
            }
            *tuner = next;
        }
    }

    /// What `station` has on air at `clock`.
    fn on_air(station: usize, clock: f64) -> Self {
        match OnAir::at(station, clock) {
            OnAir::Track { track, .. } => Self::Playing { station, track },
            OnAir::Pause { next, left } => Self::Pause {
                station,
                track: next,
                until: clock + f64::from(left),
            },
        }
    }
}

impl Broadcast {
    /// Plays the music once it has loaded. Music that doesn't load ends
    /// right away.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn start(
        mut commands: Commands,
        time: Res<Time<Real>>,
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
            let start = match OnAir::at(broadcast.station, time.elapsed_secs_f64()) {
                OnAir::Track { track, at } if track == broadcast.track => at,
                _ => 0.0,
            };
            let music =
                Panned::new(music.clone(), pan.clone()).from(Duration::from_secs_f32(start));
            commands.entity(entity).insert((
                AudioPlayer(panned.add(music)),
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
    const fn new(
        file: &'static str,
        artist: &'static str,
        title: &'static str,
        length: f32,
    ) -> Self {
        Self {
            file,
            artist,
            title,
            length,
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
