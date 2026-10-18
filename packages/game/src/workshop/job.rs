use bevy::prelude::*;
use ld60_levels::{Level, LevelError, LevelSource};

/// The repair job on the bench.
#[derive(Resource, Debug)]
pub(crate) struct Job {
    pub(crate) source: &'static LevelSource,
    pub(crate) level: Level,
    /// Set once flashed firmware passes all of the customer's tests.
    pub(crate) repaired: bool,
}

impl Job {
    pub(crate) fn load(source: &'static LevelSource) -> Result<Self, LevelError> {
        Ok(Self {
            source,
            level: source.load()?,
            repaired: false,
        })
    }
}
