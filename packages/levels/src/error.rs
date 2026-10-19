use std::fmt;

use ld60_devices::{BoardRev, DeviceError, ProgramError};
use ld60_dsl::Diagnostic;
use ld60_machine::AsmError;

use crate::Report;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LevelError {
    Parse(String),
    Write(ron::Error),
    BoardNotAllowed(BoardRev),
    NoRewrite(BoardRev),
    Assemble(Vec<AsmError>),
    Device(DeviceError),
}

impl fmt::Display for LevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "invalid level file: {message}"),
            Self::Write(_) => f.write_str("cannot write the level file"),
            Self::NoRewrite(rev) => {
                write!(f, "no rewrite for board revision {rev:?} passes the checks")
            }
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

impl std::error::Error for LevelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Write(error) => Some(error),
            Self::Device(_)
            | Self::Parse(_)
            | Self::BoardNotAllowed(_)
            | Self::NoRewrite(_)
            | Self::Assemble(_) => None,
        }
    }
}

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
    FixedFailsLineTests(Report),
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
            Self::FixedFailsLineTests(report) => {
                write!(f, "fixed.asm fails the end of line tests\n{report}")
            }
            Self::Run(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for VerifyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Buggy(error) | Self::Fixed(error) | Self::Run(error) => Some(error),
            Self::Burn(error) => Some(error),
            Self::BuggyPasses(_) | Self::FixedFails(_) | Self::FixedFailsLineTests(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenerateError {
    Compile(Vec<Diagnostic>),
    /// The working firmware fails the end of line tests.
    Broken(Report),
    Level(LevelError),
    /// No bug that the tests can show was found.
    NoBug,
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile(errors) => {
                f.write_str("firmware source does not compile")?;
                errors.iter().try_for_each(|e| write!(f, "\n  {e}"))
            }
            Self::Broken(report) => {
                write!(f, "working firmware fails the end of line tests\n{report}")
            }
            Self::Level(error) => error.fmt(f),
            Self::NoBug => f.write_str("no bug found that the tests can show"),
        }
    }
}

impl std::error::Error for GenerateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Level(error) => Some(error),
            Self::Compile(_) | Self::Broken(_) | Self::NoBug => None,
        }
    }
}

impl From<LevelError> for GenerateError {
    fn from(error: LevelError) -> Self {
        Self::Level(error)
    }
}
