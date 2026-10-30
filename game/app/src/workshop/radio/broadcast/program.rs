use bevy::prelude::*;
use rand::{RngExt, SeedableRng, rngs::SmallRng};

use super::bag::{Drawn, ShuffleBag};
use super::{PAUSE, TRACKS};

/// One station fewer than tracks, so a station that ends a track always
/// has another one to go on with.
pub(in super::super) const STATIONS: usize = TRACKS.len() - 1;
const SEED: u64 = 11;

/// What a station is playing at a moment of the broadcast clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum OnAir {
    /// `track` has played for `at` seconds.
    Track { track: usize, at: f32 },
    /// `next` comes on in `left` seconds.
    Pause { next: usize, left: f32 },
}

/// The program of every station. Each plays one track at a time, and no
/// two play the same track: at the end of a track a station takes its
/// next one from the tracks no station has, then puts its old one back.
/// The stations play on while the radio is tuned elsewhere.
#[derive(Resource, Debug)]
pub(super) struct Stations {
    bag: ShuffleBag<usize>,
    rng: SmallRng,
    stations: Vec<Slot>,
}

/// The track of one station.
#[derive(Debug)]
struct Slot {
    track: Drawn<usize>,
    /// When the track starts on the broadcast clock. Ahead of the clock
    /// during the pause before it.
    start: f64,
}

impl Default for Stations {
    /// Each station is somewhere in its first track at clock 0.
    fn default() -> Self {
        let mut rng = SmallRng::seed_from_u64(SEED);
        let mut bag = ShuffleBag::new(0..TRACKS.len());
        let stations = std::iter::from_fn(|| {
            let track = bag.draw(&mut rng)?;
            let into = rng.random::<f64>() * f64::from(TRACKS[*track.get()].length);
            Some(Slot {
                track,
                start: -into,
            })
        })
        .take(STATIONS)
        .collect();
        Self { bag, rng, stations }
    }
}

impl Stations {
    /// Plays the stations on up to `clock`.
    pub(super) fn advance(&mut self, clock: f64) {
        for slot in &mut self.stations {
            loop {
                let end = slot.start + f64::from(TRACKS[*slot.track.get()].length);
                if clock < end {
                    break;
                }
                self.bag.exchange(&mut self.rng, &mut slot.track);
                slot.start = end + f64::from(PAUSE);
            }
        }
    }

    /// What `station` has on air at `clock`, which the stations have been
    /// played on to with [`Self::advance`].
    pub(super) fn on_air(&self, station: usize, clock: f64) -> Option<OnAir> {
        let slot = self.stations.get(station)?;
        let track = *slot.track.get();
        Some(if clock < slot.start {
            OnAir::Pause {
                next: track,
                left: (slot.start - clock) as f32,
            }
        } else {
            OnAir::Track {
                track,
                at: (clock - slot.start) as f32,
            }
        })
    }

    /// Follows the clock of `Tuner` and `Broadcast`.
    pub(super) fn follow(time: Res<Time<Real>>, mut stations: ResMut<Stations>) {
        stations.advance(time.elapsed_secs_f64());
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn there_is_a_station_for_all_but_one_track() {
        assert_eq!(Stations::default().stations.len(), STATIONS);
    }

    #[test]
    fn no_two_stations_play_the_same_track() {
        let mut stations = Stations::default();
        for second in 0..10 * 3600 {
            let clock = f64::from(second);
            stations.advance(clock);
            let mut tracks: Vec<_> = (0..STATIONS)
                .map(|station| match stations.on_air(station, clock).unwrap() {
                    OnAir::Track { track, .. } | OnAir::Pause { next: track, .. } => track,
                })
                .collect();
            tracks.sort_unstable();
            tracks.dedup();
            assert_eq!(tracks.len(), STATIONS, "at {clock}");
        }
    }

    #[test]
    fn a_pause_follows_each_track_and_the_next_one_is_another() {
        let mut stations = Stations::default();
        stations.advance(0.0);
        let Some(OnAir::Track { track, at }) = stations.on_air(0, 0.0) else {
            panic!("station 0 starts in a track");
        };
        let end = f64::from(TRACKS[track].length - at);
        stations.advance(end + 1.0);
        let Some(OnAir::Pause { next, left }) = stations.on_air(0, end + 1.0) else {
            panic!("a pause follows the track");
        };
        assert_ne!(next, track);
        assert!((left - (PAUSE - 1.0)).abs() < 1e-3, "{left}");
        let after = end + f64::from(PAUSE) + 1.0;
        stations.advance(after);
        let Some(OnAir::Track { track: on, at }) = stations.on_air(0, after) else {
            panic!("the next track follows the pause");
        };
        assert_eq!(on, next);
        assert!((at - 1.0).abs() < 1e-3, "{at}");
    }

    /// The lengths match the music files, read from the granule position of
    /// the last Ogg page and the rate in the Vorbis header.
    #[test]
    fn lengths_match_the_music() {
        let music = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for track in &TRACKS {
            let bytes = std::fs::read(music.join(track.path())).unwrap();
            let last = bytes.windows(4).rposition(|w| w == b"OggS").unwrap();
            let granule = u64::from_le_bytes(bytes[last + 6..last + 14].try_into().unwrap());
            let header = bytes.windows(7).position(|w| w == b"\x01vorbis").unwrap();
            let rate = u32::from_le_bytes(bytes[header + 12..header + 16].try_into().unwrap());
            let seconds = granule as f32 / rate as f32;
            assert!(
                (seconds - track.length).abs() < 0.1,
                "{} lasts {seconds} s",
                track.path()
            );
        }
    }

    /// Tuning in halfway through a track needs a decoder that seeks.
    #[test]
    fn music_seeks() {
        use bevy::audio::{AudioSource, Decodable, Source};

        let music = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let bytes = std::fs::read(music.join(TRACKS[0].path())).unwrap();
        let mut decoder = AudioSource {
            bytes: bytes.into(),
        }
        .decoder();
        decoder
            .try_seek(std::time::Duration::from_secs(60))
            .unwrap();
    }
}
