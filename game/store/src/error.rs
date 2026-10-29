use std::{error, fmt};
#[cfg(not(target_arch = "wasm32"))]
use std::{io, path::PathBuf};

/// Why a save does not load or write.
#[derive(Debug)]
pub(crate) enum StoreError {
    #[cfg(not(target_arch = "wasm32"))]
    Read {
        path: PathBuf,
        source: io::Error,
    },
    #[cfg(not(target_arch = "wasm32"))]
    Write {
        path: PathBuf,
        source: io::Error,
    },
    /// The browser has no local storage, or keeps it from the page.
    #[cfg(target_arch = "wasm32")]
    NoStorage,
    /// The browser did not take or give the save, for example because its
    /// storage is full.
    #[cfg(target_arch = "wasm32")]
    Refused,
    Encode(ron::Error),
    /// The save is broken or from another version of the game.
    Decode(ron::error::SpannedError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Read { path, .. } => write!(f, "cannot read {}", path.display()),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Write { path, .. } => write!(f, "cannot write {}", path.display()),
            #[cfg(target_arch = "wasm32")]
            Self::NoStorage => f.write_str("the browser has no local storage"),
            #[cfg(target_arch = "wasm32")]
            Self::Refused => f.write_str("the browser refused the local storage"),
            Self::Encode(_) => f.write_str("the save does not encode"),
            Self::Decode(_) => f.write_str("the save does not decode"),
        }
    }
}

impl error::Error for StoreError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Read { source, .. } | Self::Write { source, .. } => Some(source),
            #[cfg(target_arch = "wasm32")]
            Self::NoStorage | Self::Refused => None,
            Self::Encode(source) => Some(source),
            Self::Decode(source) => Some(source),
        }
    }
}
