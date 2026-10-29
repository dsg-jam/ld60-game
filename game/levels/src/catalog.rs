use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use serde::Deserialize;

use crate::{Level, LevelError};

/// All levels: the tutorial first, then the special jobs.
#[derive(Clone, Debug)]
pub struct Catalog {
    order: Order,
    /// In the order of [`Order::paths`].
    levels: Vec<LevelSource>,
}

/// The level folders that a [`Catalog::FILE`] lists.
///
/// ```
/// use ld60_levels::Catalog;
///
/// let order = Catalog::order(
///     r#"(tutorial: ["tally2/01-clear-key"], jobs: [(level: "summa6/04-coffee", from: 6)])"#,
/// )
/// .unwrap();
/// assert_eq!(
///     order.paths().collect::<Vec<_>>(),
///     ["tally2/01-clear-key", "summa6/04-coffee"]
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Order {
    /// The first jobs of a run, in order.
    pub tutorial: Vec<String>,
    /// Hand-written jobs that come between the generated ones.
    pub jobs: Vec<Special>,
}

/// A hand-written job that comes between the generated ones.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Special {
    /// The level folder.
    pub level: String,
    /// The first generated job number it can come at.
    pub from: u32,
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

    /// The catalog of `order`, with `levels` in the order of
    /// [`Order::paths`].
    pub fn new(order: Order, levels: Vec<LevelSource>) -> Self {
        Self { order, levels }
    }

    /// The level folders listed in a [`Catalog::FILE`].
    pub fn order(source: &str) -> Result<Order, CatalogError> {
        ron::from_str(source).map_err(CatalogError::Order)
    }

    /// Reads the catalog and every level in it from `dir`.
    pub fn read(dir: &Path) -> Result<Self, CatalogError> {
        let order = Self::order(&read(dir.join(Self::FILE))?)?;
        let levels = order
            .paths()
            .map(|path| LevelSource::read(dir, path.into()))
            .collect::<Result<_, _>>()?;
        Ok(Self { order, levels })
    }

    /// Every level, the tutorial first.
    pub fn levels(&self) -> &[LevelSource] {
        &self.levels
    }

    /// The first jobs of a run.
    pub fn tutorial(&self) -> &[LevelSource] {
        &self.levels[..self.order.tutorial.len().min(self.levels.len())]
    }

    /// The hand-written jobs that come between the generated ones, each with
    /// [`Special::from`].
    pub fn specials(&self) -> impl Iterator<Item = (&LevelSource, u32)> {
        let tutorial = self.tutorial().len();
        self.levels[tutorial..]
            .iter()
            .zip(self.order.jobs.iter().map(|special| special.from))
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

impl Order {
    /// Every level folder, the tutorial first.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.tutorial
            .iter()
            .map(String::as_str)
            .chain(self.jobs.iter().map(|special| special.level.as_str()))
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

    /// Reads the level in `folder`, which need not be in a catalog.
    pub fn read_folder(folder: &Path) -> Result<Self, CatalogError> {
        let file = |name| read(folder.join(name));
        Ok(Self {
            level: file(Self::LEVEL_FILE)?,
            buggy: file(Self::BUGGY_FILE)?,
            fixed: file(Self::FIXED_FILE)?,
            path: folder.display().to_string(),
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
