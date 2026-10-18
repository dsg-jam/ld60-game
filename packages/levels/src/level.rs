use ld60_devices::{ChipSpec, DeviceKind, Fault, RomKind};
use serde::{Deserialize, Serialize};

use crate::LevelError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub id: String,
    pub title: String,
    pub device: DeviceKind,
    pub chip: ChipSpec,
    /// Printed on the sticker under the device and on the job ticket.
    pub serial: String,
    /// Notes from the intake bench, shown on the job ticket.
    #[serde(default)]
    pub remarks: Vec<String>,
    pub rom: RomSpec,
    #[serde(default)]
    pub faults: Vec<Fault>,
    pub customer: Customer,
    pub tests: Vec<TestCase>,
    pub reward: u32,
    /// Where `buggy.asm` and `fixed.asm` are generated from, if they are.
    #[serde(default)]
    pub firmware: Option<FirmwareSource>,
}

/// A firmware source file and the switches a level turns on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareSource {
    /// Path relative to [`DATA_DIR`](crate::DATA_DIR).
    pub source: String,
    #[serde(default)]
    pub switches: Vec<String>,
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
