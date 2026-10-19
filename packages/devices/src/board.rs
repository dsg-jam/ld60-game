use std::ops::RangeInclusive;
use std::str::FromStr;

use crate::{DeviceKind, UnknownBoardRev};

/// Revision of the board around the chip. Revision A is the original one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BoardRev {
    #[default]
    A,
    B,
    C,
}

impl BoardRev {
    pub const ALL: [Self; 3] = [Self::A, Self::B, Self::C];
}

impl FromStr for BoardRev {
    type Err = UnknownBoardRev;

    /// Parses the letter, like `B` or `b`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|rev| format!("{rev:?}").eq_ignore_ascii_case(text))
            .ok_or_else(|| UnknownBoardRev(text.into()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Annunciator {
    Minus,
    Error,
    Open,
    Shut,
}

impl Annunciator {
    pub const ALL: [Self; 4] = [Self::Minus, Self::Error, Self::Open, Self::Shut];

    /// What the display glass prints for it, and how a [`Readout`] shows it
    /// as text.
    ///
    /// [`Readout`]: crate::Readout
    pub fn mark(self) -> &'static str {
        match self {
            Self::Minus => "-",
            Self::Error => "E",
            Self::Open => "OPEN",
            Self::Shut => "SHUT",
        }
    }
}

/// The circuit board around the chip. Board revisions of one model wire the
/// keys, the display and the annunciators differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Board {
    pub kind: DeviceKind,
    pub rev: BoardRev,
    /// RAM cell of the leftmost digit. The other digits follow to the right.
    pub display: u8,
    /// Each key and the code the firmware reads from `KEY`.
    pub keys: &'static [(char, u8)],
    /// Each annunciator and the `ANN` bit that lights it.
    pub annunciators: &'static [(Annunciator, u8)],
}

impl Board {
    /// The `ANN` bit that lights `annunciator`, if the board has it.
    pub fn ann_bit(&self, annunciator: Annunciator) -> Option<u8> {
        self.annunciators
            .iter()
            .find(|(a, _)| *a == annunciator)
            .map(|&(_, bit)| bit)
    }

    /// RAM cells the display shows, from left to right.
    pub fn display_cells(&self) -> RangeInclusive<usize> {
        let first = usize::from(self.display);
        first..=first + self.kind.digits() - 1
    }

    pub fn key_code(&self, key: char) -> Option<u8> {
        self.keys
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&key))
            .map(|&(_, code)| code)
    }

    /// Printed on the board and in the serial number, like `6B`.
    pub fn name(&self) -> String {
        format!("{}{:?}", self.kind.board_prefix(), self.rev)
    }
}
