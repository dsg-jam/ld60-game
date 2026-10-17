use std::fmt;

use devices::DeviceError;
use machine::AsmError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LevelError {
    Parse(String),
    Assemble(Vec<AsmError>),
    Device(DeviceError),
}

impl fmt::Display for LevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "invalid level file: {message}"),
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
