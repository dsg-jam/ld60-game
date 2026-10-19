use std::fmt;

use crate::{DeviceKind, IsaKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceError {
    ChipMismatch { device: DeviceKind, chip: IsaKind },
    RomSize { expected: usize, actual: usize },
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChipMismatch { device, chip } => {
                write!(f, "{device:?} cannot use a {chip:?} chip")
            }
            Self::RomSize { expected, actual } => {
                write!(f, "ROM has {actual} words, expected {expected}")
            }
        }
    }
}

impl std::error::Error for DeviceError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramError {
    MaskRom,
    SizeMismatch,
    BitConflict { addr: usize },
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaskRom => f.write_str("a mask ROM cannot be programmed"),
            Self::SizeMismatch => f.write_str("the image does not match the ROM size"),
            Self::BitConflict { addr } => {
                write!(
                    f,
                    "0x{addr:02X} needs a bit set from 0 to 1. Erase the chip first."
                )
            }
        }
    }
}

impl std::error::Error for ProgramError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunError {
    Timeout { steps: u32 },
    UnknownKey(char),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout { steps } => write!(f, "no halt after {steps} steps"),
            Self::UnknownKey(key) => write!(f, "the device has no '{key}' key"),
        }
    }
}

impl std::error::Error for RunError {}

/// The text that did not name a board revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownBoardRev(pub String);

impl fmt::Display for UnknownBoardRev {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown board revision '{}', use A, B or C", self.0)
    }
}

impl std::error::Error for UnknownBoardRev {}

/// The text that did not name a device model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownDeviceKind(pub String);

impl fmt::Display for UnknownDeviceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<String> = DeviceKind::ALL
            .iter()
            .map(|kind| format!("{kind:?}").to_lowercase())
            .collect();
        write!(f, "unknown device '{}', use {}", self.0, names.join(" or "))
    }
}

impl std::error::Error for UnknownDeviceKind {}
