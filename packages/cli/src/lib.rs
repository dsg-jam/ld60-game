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

use ld60_levels::{CATALOG, LevelSource};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn find_level(path: &str) -> Result<&'static LevelSource> {
    CATALOG
        .iter()
        .find(|s| s.path == path)
        .ok_or_else(|| format!("unknown level '{path}'. Run 'cli list' to see all levels.").into())
}
