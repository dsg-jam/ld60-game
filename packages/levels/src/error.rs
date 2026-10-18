use std::fmt;

use ld60_devices::{BoardRev, DeviceError, ProgramError};
use ld60_machine::AsmError;

use crate::Report;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LevelError {
    Parse(String),
    BoardNotAllowed(BoardRev),
    Assemble(Vec<AsmError>),
    Device(DeviceError),
}

impl fmt::Display for LevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "invalid level file: {message}"),
            Self::BoardNotAllowed(rev) => {
                write!(f, "the job does not allow board revision {rev:?}")
            }
            Self::Assemble(errors) => {
                f.write_str("assembly failed")?;
                errors.iter().try_for_each(|e| write!(f, "\n  {e}"))
            }
            Self::Device(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for LevelError {}

impl From<DeviceError> for LevelError {
    fn from(error: DeviceError) -> Self {
        Self::Device(error)
    }
}

impl From<Vec<AsmError>> for LevelError {
    fn from(errors: Vec<AsmError>) -> Self {
        Self::Assemble(errors)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyError {
    Buggy(LevelError),
    BuggyPasses(Report),
    Burn(ProgramError),
    Fixed(LevelError),
    FixedFails(Report),
    Run(LevelError),
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buggy(error) => write!(f, "buggy.asm: {error}"),
            Self::BuggyPasses(report) => write!(f, "buggy.asm passes all tests\n{report}"),
            Self::Burn(error) => write!(f, "fixed.asm cannot be burned over the chip: {error}"),
            Self::Fixed(error) => write!(f, "fixed.asm: {error}"),
            Self::FixedFails(report) => write!(f, "fixed.asm fails\n{report}"),
            Self::Run(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for VerifyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Buggy(error) | Self::Fixed(error) | Self::Run(error) => Some(error),
            Self::Burn(error) => Some(error),
            Self::BuggyPasses(_) | Self::FixedFails(_) => None,
        }
    }
}
