//! Keeps saves between sessions.
//!
//! # Features
//!
//! - `test-support`: adds `Store::in_temp` for tests in other crates.

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(not(target_arch = "wasm32"))]
mod disk;
mod error;

use std::error::Error;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use bevy::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

use self::error::StoreError;

/// Keeps saves between sessions: RON files in the player's data folder on
/// native builds, and the browser's local storage on the web.
///
/// A save that does not load or write only logs an error, so the game still
/// runs without it.
#[derive(Resource, Debug)]
#[cfg_attr(target_arch = "wasm32", derive(Default))]
pub struct Store {
    /// No folder means the game does not save.
    #[cfg(not(target_arch = "wasm32"))]
    dir: Option<PathBuf>,
}

/// What a save holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// The run of the shop.
    Shop,
    Settings,
}

impl Store {
    /// The value in `slot`, if there is one and it loads.
    ///
    /// A save that does not decode is kept as a copy next to it, so the next
    /// save does not lose it.
    pub fn load<T: DeserializeOwned>(&self, slot: Slot) -> Option<T> {
        let loaded = self.read(slot.key()).and_then(|text| {
            let Some(text) = text else {
                return Ok(None);
            };
            ron::from_str(&text).map(Some).map_err(|err| {
                self.keep_broken(slot, &text);
                StoreError::Decode(err)
            })
        });
        match loaded {
            Ok(value) => value,
            Err(err) => {
                error!(
                    error = &err as &dyn Error,
                    slot = slot.key(),
                    "save does not load"
                );
                None
            }
        }
    }

    /// Puts `value` in `slot`, over what was there.
    pub fn save<T: Serialize>(&self, slot: Slot, value: &T) {
        let written = ron::to_string(value)
            .map_err(StoreError::Encode)
            .and_then(|text| self.write(slot.key(), &text));
        if let Err(err) = written {
            error!(
                error = &err as &dyn Error,
                slot = slot.key(),
                "save does not write"
            );
        }
    }

    fn keep_broken(&self, slot: Slot, text: &str) {
        if let Err(err) = self.write(slot.broken_key(), text) {
            error!(
                error = &err as &dyn Error,
                slot = slot.key(),
                "broken save is not kept"
            );
        }
    }
}

impl Slot {
    const fn key(self) -> &'static str {
        match self {
            Self::Shop => "shop",
            Self::Settings => "settings",
        }
    }

    /// Where a save that does not decode is kept.
    const fn broken_key(self) -> &'static str {
        match self {
            Self::Shop => "shop.bad",
            Self::Settings => "settings.bad",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
    struct Note {
        text: String,
        count: u32,
    }

    fn note() -> Note {
        Note {
            text: "flash it".to_owned(),
            count: 3,
        }
    }

    #[test]
    fn a_save_loads_back() {
        let store = Store::in_temp("loads-back");
        assert_eq!(store.load::<Note>(Slot::Shop), None);
        store.save(Slot::Shop, &note());
        assert_eq!(store.load(Slot::Shop), Some(note()));
        assert_eq!(store.load::<Note>(Slot::Settings), None);
    }

    #[test]
    fn a_new_save_replaces_the_old_one() {
        let store = Store::in_temp("replaces");
        store.save(Slot::Settings, &note());
        let newer = Note { count: 4, ..note() };
        store.save(Slot::Settings, &newer);
        assert_eq!(store.load(Slot::Settings), Some(newer));
    }

    #[test]
    fn a_broken_save_loads_as_nothing() {
        let store = Store::in_temp("broken");
        store.write(Slot::Shop.key(), "(text: 3").unwrap();
        assert_eq!(store.load::<Note>(Slot::Shop), None);
        store.save(Slot::Shop, &note());
        assert_eq!(store.load(Slot::Shop), Some(note()));
        assert_eq!(
            store.read(Slot::Shop.broken_key()).unwrap().as_deref(),
            Some("(text: 3")
        );
    }

    #[test]
    fn without_a_folder_nothing_saves() {
        let store = Store { dir: None };
        store.save(Slot::Shop, &note());
        assert_eq!(store.load::<Note>(Slot::Shop), None);
    }
}
