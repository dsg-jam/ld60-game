use crate::{Level, LevelError};

macro_rules! level {
    ($path:literal) => {
        LevelSource {
            path: $path,
            level: include_str!(concat!("../data/", $path, "/level.ron")),
            buggy: include_str!(concat!("../data/", $path, "/buggy.asm")),
            fixed: include_str!(concat!("../data/", $path, "/fixed.asm")),
        }
    };
}

pub const CATALOG: &[LevelSource] = &[
    level!("summa6/01-strange-symbols"),
    level!("summa6/02-last-digit"),
];

/// Level files, embedded at compile time so they also work on the web.
#[derive(Clone, Copy, Debug)]
pub struct LevelSource {
    pub path: &'static str,
    pub level: &'static str,
    pub buggy: &'static str,
    pub fixed: &'static str,
}

impl LevelSource {
    pub fn load(&self) -> Result<Level, LevelError> {
        Level::from_ron(self.level)
    }
}
