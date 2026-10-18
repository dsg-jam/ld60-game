use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use crate::{Level, LevelError};

/// All levels in the order they are played.
#[derive(Clone, Debug)]
pub struct Catalog {
    levels: Vec<LevelSource>,
}

/// The files of one level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelSource {
    /// The folder of the level, relative to the data directory.
    pub path: String,
    pub level: String,
    pub buggy: String,
    pub fixed: String,
}

#[derive(Debug)]
pub enum CatalogError {
    Read { path: PathBuf, source: io::Error },
    Order(ron::error::SpannedError),
}

impl Catalog {
    /// Lists the level folders in play order.
    pub const FILE: &str = "catalog.ron";

    pub const fn new(levels: Vec<LevelSource>) -> Self {
        Self { levels }
    }

    /// The level folders listed in a [`Catalog::FILE`].
    pub fn order(source: &str) -> Result<Vec<String>, CatalogError> {
        ron::from_str(source).map_err(CatalogError::Order)
    }

    /// Reads the catalog and every level in it from `dir`.
    pub fn read(dir: &Path) -> Result<Self, CatalogError> {
        let levels = Self::order(&read(dir.join(Self::FILE))?)?
            .into_iter()
            .map(|path| LevelSource::read(dir, path))
            .collect::<Result<_, _>>()?;
        Ok(Self { levels })
    }

    pub fn levels(&self) -> &[LevelSource] {
        &self.levels
    }

    /// The level in the folder `path`.
    pub fn find(&self, path: &str) -> Option<&LevelSource> {
        self.levels.iter().find(|source| source.path == path)
    }

    /// Where the level in the folder `path` is in the play order.
    pub fn position(&self, path: &str) -> Option<usize> {
        self.levels.iter().position(|source| source.path == path)
    }
}

#[cfg(test)]
impl Catalog {
    /// The levels in the repository.
    pub(crate) fn in_repository() -> Self {
        Self::read(&Path::new(env!("LD60_ASSETS")).join("levels")).unwrap()
    }
}

impl LevelSource {
    pub const LEVEL_FILE: &str = "level.ron";
    pub const BUGGY_FILE: &str = "buggy.asm";
    pub const FIXED_FILE: &str = "fixed.asm";

    /// The path of `file` in the level folder `path`, relative to the data
    /// directory.
    pub fn file(path: &str, file: &str) -> String {
        format!("{path}/{file}")
    }

    /// Reads the level in the folder `path` from the data directory `dir`.
    pub fn read(dir: &Path, path: String) -> Result<Self, CatalogError> {
        let file = |name| read(dir.join(Self::file(&path, name)));
        Ok(Self {
            level: file(Self::LEVEL_FILE)?,
            buggy: file(Self::BUGGY_FILE)?,
            fixed: file(Self::FIXED_FILE)?,
            path,
        })
    }

    pub fn load(&self) -> Result<Level, LevelError> {
        Level::from_ron(&self.level)
    }
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, .. } => write!(f, "cannot read {}", path.display()),
            Self::Order(_) => write!(f, "invalid {}", Catalog::FILE),
        }
    }
}

impl std::error::Error for CatalogError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Order(source) => Some(source),
        }
    }
}

fn read(path: PathBuf) -> Result<String, CatalogError> {
    fs::read_to_string(&path).map_err(|source| CatalogError::Read { path, source })
}
