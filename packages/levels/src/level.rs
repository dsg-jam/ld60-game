use ld60_devices::{Board, BoardRev, ChipSpec, DeviceKind, Fault, RomKind};
use serde::{Deserialize, Serialize};

use crate::LevelError;

/// A repair job.
///
/// Its text is in a Fluent message named after [`Level::id`]. The attributes
/// of that message are [`Level::CUSTOMER`], [`Level::COMPLAINT`] and one
/// [`Level::remark`] per remark. They get the arguments of
/// [`Level::text_args`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub id: String,
    pub title: String,
    pub device: DeviceKind,
    /// The board in the device. The firmware files are written for it.
    #[serde(default)]
    pub board: BoardRev,
    /// Boards a variant may move the job to. Empty allows every board.
    #[serde(default)]
    pub boards: Vec<BoardRev>,
    /// Keeps the firmware layout as written. Variants then only move the job
    /// to another board.
    #[serde(default)]
    pub keep_layout: bool,
    pub chip: ChipSpec,
    /// The unit number. [`Level::serial`] puts the board in front of it.
    pub unit: String,
    /// Notes from the intake bench, shown on the job ticket. Each is the
    /// key of an attribute of the level's message, see [`Level::remark`].
    #[serde(default)]
    pub remarks: Vec<String>,
    pub rom: RomSpec,
    #[serde(default)]
    pub faults: Vec<Fault>,
    pub tests: Vec<TestCase>,
    pub reward: u32,
    /// Where `buggy.asm` and `fixed.asm` are generated from, if they are.
    #[serde(default)]
    pub firmware: Option<FirmwareSource>,
}

/// A firmware source file and the switches a level turns on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareSource {
    /// Path relative to the folder of all levels.
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
pub struct TestCase {
    /// Keys pressed after power on. Whitespace is ignored.
    pub keys: String,
    pub display: String,
}

impl Level {
    /// The attribute that names the customer.
    pub const CUSTOMER: &str = "customer";
    /// The attribute with what the customer says is wrong.
    pub const COMPLAINT: &str = "complaint";

    /// The attribute for the remark `key`.
    pub fn remark(key: &str) -> String {
        format!("remark-{key}")
    }

    /// The arguments of the level's attributes: the board, like `6A`, as
    /// `$board`.
    pub fn text_args(&self) -> [(&'static str, String); 1] {
        [("board", self.board().name())]
    }

    /// Printed on the sticker under the device and on the job ticket.
    pub fn serial(&self) -> String {
        format!("{}-{}", self.board().name(), self.unit)
    }

    pub fn from_ron(source: &str) -> Result<Self, LevelError> {
        ron::from_str(source).map_err(|e| LevelError::Parse(e.to_string()))
    }

    pub fn board(&self) -> &'static Board {
        self.device.board(self.board)
    }

    /// Boards a variant may move the job to.
    ///
    /// Faded ROM bits sit at fixed addresses, so these jobs keep their board.
    pub fn allowed_boards(&self) -> Vec<BoardRev> {
        if self.has_rom_damage() {
            vec![self.board]
        } else if self.boards.is_empty() {
            BoardRev::ALL.to_vec()
        } else {
            self.boards.clone()
        }
    }

    /// Faded ROM bits sit at fixed addresses, so these jobs keep their layout too.
    pub fn keeps_layout(&self) -> bool {
        self.keep_layout || self.has_rom_damage()
    }

    pub fn has_rom_damage(&self) -> bool {
        self.faults
            .iter()
            .any(|f| matches!(f, Fault::FadedRomBits { .. }))
    }
}
