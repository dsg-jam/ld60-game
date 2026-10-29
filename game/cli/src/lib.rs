//! Terminal tools to try levels and firmware before the game exists.

pub mod check;
pub mod generate;
pub mod list;
pub mod listing;
pub mod play;
pub mod procedural;
mod render;
pub mod seed;
mod session;
pub mod text;

pub use session::{Session, Stop};

use std::path::Path;

use ld60_devices::BoardRev;
use ld60_levels::{Catalog, LevelSource, Variant};

/// The level folders in the repository.
pub const LEVELS: &str = concat!(env!("LD60_ASSETS"), "/levels");

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn catalog() -> Result<Catalog> {
    Ok(Catalog::read(Path::new(LEVELS))?)
}

/// The level in the catalog folder `path`, or in the folder at `path`, like
/// one that `cli gen --procedural` wrote.
pub fn find_level(path: &str) -> Result<LevelSource> {
    if let Some(source) = catalog()?.find(path) {
        return Ok(source.clone());
    }
    let folder = Path::new(path);
    if folder.join(LevelSource::LEVEL_FILE).is_file() {
        return Ok(LevelSource::read_folder(folder)?);
    }
    Err(format!("unknown level '{path}'. Run 'cli list' to see all levels.").into())
}

/// The level as written, or the variant for `seed`. A `board` other than the
/// level's own forces a variant on that board, with seed 0 if none is given.
pub fn find_variant(path: &str, seed: Option<u64>, board: Option<BoardRev>) -> Result<Variant> {
    let source = &find_level(path)?;
    let original = Variant::original(source)?;
    Ok(match (seed, board) {
        (None, None) => original,
        (None, Some(board)) if board == original.level.board => original,
        (Some(seed), None) => Variant::new(source, seed)?,
        (seed, Some(board)) => Variant::with_board(source, seed.unwrap_or(0), board)?,
    })
}
