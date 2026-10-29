use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use bevy::prelude::*;

use super::{Store, StoreError};

/// The folder of the game in the player's data folder.
const FOLDER: &str = "ld60";

impl Default for Store {
    fn default() -> Self {
        let dir = Self::data_dir().map(|dir| dir.join(FOLDER));
        if dir.is_none() {
            warn!("there is no data folder, so the game does not save");
        }
        Self { dir }
    }
}

impl Store {
    /// The folder where the player's programs keep their data.
    fn data_dir() -> Option<PathBuf> {
        let var = |name| env::var_os(name).filter(|value| !value.is_empty());
        if cfg!(windows) {
            var("APPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            var("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
        } else {
            var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| var("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        }
    }

    fn path(&self, key: &str) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join(format!("{key}.ron")))
    }

    pub(super) fn read(&self, key: &str) -> Result<Option<String>, StoreError> {
        let Some(path) = self.path(key) else {
            return Ok(None);
        };
        match fs::read_to_string(&path) {
            Ok(text) => Ok(Some(text)),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
            Err(source) => Err(StoreError::Read { path, source }),
        }
    }

    /// Writes next to the save first, so a crash never leaves half a save.
    pub(super) fn write(&self, key: &str, text: &str) -> Result<(), StoreError> {
        let (Some(dir), Some(path)) = (&self.dir, self.path(key)) else {
            return Ok(());
        };
        let draft = path.with_extension("ron.new");
        fs::create_dir_all(dir)
            .and_then(|()| fs::write(&draft, text))
            .map_err(|source| StoreError::Write {
                path: draft.clone(),
                source,
            })?;
        fs::rename(&draft, &path).map_err(|source| StoreError::Write { path, source })
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Store {
    /// A store in a new, empty folder for the test `name`.
    pub fn in_temp(name: &str) -> Self {
        let dir = env::temp_dir().join(format!("ld60-store-{}-{name}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        Self { dir: Some(dir) }
    }
}
