use super::{PAUSE, TRACKS};

/// What a station is playing at a moment of the broadcast clock.
///
/// Every station plays all the tracks in turn with a pause after each.
/// Station n starts the clock with track n, so each station is a track
/// ahead of the one before.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum OnAir {
    /// `track` has played for `at` seconds.
    Track { track: usize, at: f32 },
    /// `next` comes on in `left` seconds.
    Pause { next: usize, left: f32 },
}

impl OnAir {
    /// On `station` at `clock` seconds.
    pub(super) fn at(station: usize, clock: f64) -> Self {
        let slot = |track: &super::Track| f64::from(track.length + PAUSE);
        let lead: f64 = TRACKS[..station].iter().map(slot).sum();
        let program: f64 = TRACKS.iter().map(slot).sum();
        let mut time = (clock + lead).rem_euclid(program);
        for (track, entry) in TRACKS.iter().enumerate() {
            let length = f64::from(entry.length);
            if time < length {
                return Self::Track {
                    track,
                    at: time as f32,
                };
            }
            time -= length;
            if time < f64::from(PAUSE) || track + 1 == TRACKS.len() {
                return Self::Pause {
                    next: (track + 1) % TRACKS.len(),
                    left: (f64::from(PAUSE) - time).max(0.0) as f32,
                };
            }
            time -= f64::from(PAUSE);
        }
        unreachable!("the program has tracks")
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn stations_start_with_their_own_track() {
        for station in 0..TRACKS.len() {
            assert_eq!(
                OnAir::at(station, 0.0),
                OnAir::Track {
                    track: station,
                    at: 0.0
                }
            );
        }
    }

    #[test]
    fn a_pause_follows_each_track() {
        let after = f64::from(TRACKS[0].length) + 1.0;
        assert_eq!(
            OnAir::at(0, after),
            OnAir::Pause {
                next: 1,
                left: PAUSE - 1.0
            }
        );
        assert_eq!(
            OnAir::at(0, after + f64::from(PAUSE)),
            OnAir::Track { track: 1, at: 1.0 }
        );
    }

    #[test]
    fn each_station_is_a_track_ahead() {
        for (station, track) in TRACKS.iter().enumerate().take(TRACKS.len() - 1) {
            let slot = f64::from(track.length + PAUSE);
            for clock in [0.0, 100.0, 1000.0, 5000.0] {
                assert_eq!(
                    OnAir::at(station + 1, clock),
                    OnAir::at(station, clock + slot)
                );
            }
        }
    }

    /// After the first track, the next station is not starting the same
    /// track over.
    #[test]
    fn the_next_station_is_not_a_rerun() {
        let clock = f64::from(TRACKS[0].length + PAUSE);
        assert_eq!(OnAir::at(0, clock), OnAir::Track { track: 1, at: 0.0 });
        assert_ne!(OnAir::at(1, clock), OnAir::Track { track: 1, at: 0.0 });
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
