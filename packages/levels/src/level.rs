use ld60_devices::{ChipSpec, DeviceKind, Fault, RomKind};
use serde::{Deserialize, Serialize};

use crate::LevelError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub id: String,
    pub title: String,
    pub device: DeviceKind,
    pub chip: ChipSpec,
    pub rom: RomSpec,
    #[serde(default)]
    pub faults: Vec<Fault>,
    pub customer: Customer,
    pub tests: Vec<TestCase>,
    pub reward: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RomSpec {
    pub kind: RomKind,
    /// If false, the fix must be burned over the existing chip.
    pub blank_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Customer {
    pub name: String,
    pub complaint: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCase {
    /// Keys pressed after power on. Whitespace is ignored.
    pub keys: String,
    pub display: String,
}

impl Level {
    pub fn from_ron(source: &str) -> Result<Self, LevelError> {
        ron::from_str(source).map_err(|e| LevelError::Parse(e.to_string()))
    }
}
