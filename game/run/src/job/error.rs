use std::{error, fmt};

use ld60_levels::{GenerateError, LevelError};

/// Why the next job does not come on the bench.
#[derive(Debug)]
pub enum JobError {
    /// A hand-written job does not load.
    Level(LevelError),
    /// No generated job was made.
    Generate(GenerateError),
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Level(_) => f.write_str("the level does not load"),
            Self::Generate(_) => f.write_str("the job does not generate"),
        }
    }
}

impl error::Error for JobError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Level(source) => Some(source),
            Self::Generate(source) => Some(source),
        }
    }
}

impl From<LevelError> for JobError {
    fn from(error: LevelError) -> Self {
        Self::Level(error)
    }
}

impl From<GenerateError> for JobError {
    fn from(error: GenerateError) -> Self {
        Self::Generate(error)
    }
}
