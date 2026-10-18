//! Terminal tools to try levels and firmware before the game exists.

pub mod check;
pub mod generate;
pub mod list;
pub mod listing;
pub mod play;
mod render;
pub mod seed;
mod session;

pub use session::{Session, Stop};

use ld60_devices::BoardRev;
use ld60_levels::{CATALOG, LevelSource, Variant};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn find_level(path: &str) -> Result<&'static LevelSource> {
    CATALOG
        .iter()
        .find(|s| s.path == path)
        .ok_or_else(|| format!("unknown level '{path}'. Run 'cli list' to see all levels.").into())
}

/// The level as written, or the variant for `seed`. A `board` other than the
/// level's own forces a variant on that board, with seed 0 if none is given.
pub fn find_variant(path: &str, seed: Option<u64>, board: Option<BoardRev>) -> Result<Variant> {
    let source = find_level(path)?;
    let original = Variant::original(source)?;
    Ok(match (seed, board) {
        (None, None) => original,
        (None, Some(board)) if board == original.level.board => original,
        (Some(seed), None) => Variant::new(source, seed)?,
        (seed, Some(board)) => Variant::with_board(source, seed.unwrap_or(0), board)?,
    })
}
