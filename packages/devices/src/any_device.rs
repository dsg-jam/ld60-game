use std::str::FromStr;

use ld60_machine::{Cpu, Step};

use crate::{
    Annunciator, Board, BoardRev, CalcoDevice, ChipSpec, Device, DeviceError, IsaKind, Readout,
    Rom, UnknownDeviceKind,
};

macro_rules! each {
    ($self:expr, $device:ident => $body:expr) => {
        match $self {
            AnyDevice::Calco($device) => $body,
        }
    };
}

const DIGIT_KEYS: [(char, u8); 10] = [
    ('0', 0x00),
    ('1', 0x01),
    ('2', 0x02),
    ('3', 0x03),
    ('4', 0x04),
    ('5', 0x05),
    ('6', 0x06),
    ('7', 0x07),
    ('8', 0x08),
    ('9', 0x09),
];

const TALLY2_BOARDS: [Board; 3] = [
    Board {
        kind: DeviceKind::Tally2,
        rev: BoardRev::A,
        display: 1,
        keys: &[('+', 0x0A), ('C', 0x0D)],
        annunciators: &[],
    },
    Board {
        kind: DeviceKind::Tally2,
        rev: BoardRev::B,
        display: 4,
        keys: &[('+', 0x0B), ('C', 0x0C)],
        annunciators: &[],
    },
    Board {
        kind: DeviceKind::Tally2,
        rev: BoardRev::C,
        display: 0,
        keys: &[('+', 0x01), ('C', 0x02)],
        annunciators: &[],
    },
];

const SUMMA6_BOARDS: [Board; 3] = [
    Board {
        kind: DeviceKind::Summa6,
        rev: BoardRev::A,
        display: 1,
        keys: &summa6_keys([0x0A, 0x0B, 0x0C, 0x0D]),
        annunciators: &[(Annunciator::Minus, 0), (Annunciator::Error, 1)],
    },
    Board {
        kind: DeviceKind::Summa6,
        rev: BoardRev::B,
        display: 2,
        keys: &summa6_keys([0x0C, 0x0D, 0x0E, 0x0F]),
        annunciators: &[(Annunciator::Minus, 1), (Annunciator::Error, 0)],
    },
    Board {
        kind: DeviceKind::Summa6,
        rev: BoardRev::C,
        display: 8,
        keys: &summa6_keys([0x0B, 0x0A, 0x0D, 0x0C]),
        annunciators: &[(Annunciator::Minus, 2), (Annunciator::Error, 3)],
    },
];

const LOCK4_BOARDS: [Board; 3] = [
    Board {
        kind: DeviceKind::Lock4,
        rev: BoardRev::A,
        display: 1,
        keys: &lock4_keys([0x0A, 0x0B, 0x0C]),
        annunciators: &[
            (Annunciator::Open, 0),
            (Annunciator::Shut, 1),
            (Annunciator::Error, 2),
        ],
    },
    Board {
        kind: DeviceKind::Lock4,
        rev: BoardRev::B,
        display: 6,
        keys: &lock4_keys([0x0E, 0x0D, 0x0F]),
        annunciators: &[
            (Annunciator::Open, 2),
            (Annunciator::Shut, 0),
            (Annunciator::Error, 1),
        ],
    },
    Board {
        kind: DeviceKind::Lock4,
        rev: BoardRev::C,
        display: 11,
        keys: &lock4_keys([0x0B, 0x0C, 0x0A]),
        annunciators: &[
            (Annunciator::Open, 1),
            (Annunciator::Shut, 3),
            (Annunciator::Error, 0),
        ],
    },
];

const SCORE5_BOARDS: [Board; 3] = [
    Board {
        kind: DeviceKind::Score5,
        rev: BoardRev::A,
        display: 1,
        keys: &score5_keys([0x0A, 0x0B, 0x0C, 0x0D]),
        annunciators: &[],
    },
    Board {
        kind: DeviceKind::Score5,
        rev: BoardRev::B,
        display: 9,
        keys: &score5_keys([0x0D, 0x0F, 0x0E, 0x0C]),
        annunciators: &[],
    },
    Board {
        kind: DeviceKind::Score5,
        rev: BoardRev::C,
        display: 4,
        keys: &score5_keys([0x0C, 0x0A, 0x0F, 0x0B]),
        annunciators: &[],
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeviceKind {
    /// 2-digit tally counter.
    Tally2,
    /// 6-digit adding machine.
    Summa6,
    /// Door lock with a 4-digit code.
    Lock4,
    /// Scoreboard with two 2-digit scores and the period between them.
    Score5,
}

impl DeviceKind {
    pub const ALL: [Self; 4] = [Self::Tally2, Self::Summa6, Self::Lock4, Self::Score5];

    pub fn board(self, rev: BoardRev) -> &'static Board {
        &self.boards()[rev as usize]
    }

    /// Starts the name of each board of the model, like the `6` in `6B`.
    pub fn board_prefix(self) -> &'static str {
        match self {
            Self::Tally2 => "2",
            Self::Summa6 => "6",
            Self::Lock4 => "4",
            Self::Score5 => "5",
        }
    }

    /// Every board revision of the model, in the order of [`BoardRev::ALL`].
    pub fn boards(self) -> &'static [Board; 3] {
        match self {
            Self::Tally2 => &TALLY2_BOARDS,
            Self::Summa6 => &SUMMA6_BOARDS,
            Self::Lock4 => &LOCK4_BOARDS,
            Self::Score5 => &SCORE5_BOARDS,
        }
    }

    pub fn digits(self) -> usize {
        match self {
            Self::Tally2 => 2,
            Self::Summa6 => 6,
            Self::Lock4 => 4,
            Self::Score5 => 5,
        }
    }

    pub fn isa(self) -> IsaKind {
        match self {
            Self::Tally2 | Self::Summa6 | Self::Lock4 | Self::Score5 => IsaKind::Calco,
        }
    }
}

impl FromStr for DeviceKind {
    type Err = UnknownDeviceKind;

    /// Parses the model name, like `summa6` or `Summa6`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| format!("{kind:?}").eq_ignore_ascii_case(text))
            .ok_or_else(|| UnknownDeviceKind(text.into()))
    }
}

#[derive(Clone, Debug)]
pub enum AnyDevice {
    Calco(CalcoDevice),
}

impl AnyDevice {
    pub fn new(board: &'static Board, chip: ChipSpec, rom: Rom) -> Result<Self, DeviceError> {
        if chip.isa_kind() != board.kind.isa() {
            return Err(DeviceError::ChipMismatch {
                device: board.kind,
                chip: chip.isa_kind(),
            });
        }
        let ChipSpec::Calco(info) = chip;
        CalcoDevice::new(board, info, rom).map(Self::Calco)
    }
}

impl Device for AnyDevice {
    fn board(&self) -> &'static Board {
        each!(self, d => d.board())
    }

    fn kind(&self) -> DeviceKind {
        each!(self, d => d.kind())
    }

    fn chip(&self) -> ChipSpec {
        each!(self, d => d.chip())
    }

    fn cpu(&self) -> &dyn Cpu {
        each!(self, d => d.cpu())
    }

    fn cpu_mut(&mut self) -> &mut dyn Cpu {
        each!(self, d => d.cpu_mut())
    }

    fn rom(&self) -> &Rom {
        each!(self, d => d.rom())
    }

    fn rom_mut(&mut self) -> &mut Rom {
        each!(self, d => d.rom_mut())
    }

    fn power_on(&mut self, seed: u64) {
        each!(self, d => d.power_on(seed))
    }

    fn key_code(&self, key: char) -> Option<u8> {
        each!(self, d => d.key_code(key))
    }

    fn press(&mut self, code: u8) {
        each!(self, d => d.press(code))
    }

    fn step(&mut self) -> Step {
        each!(self, d => d.step())
    }

    fn readout(&self) -> Readout {
        each!(self, d => d.readout())
    }
}

/// Digits 0 to 9 send their value. The other keys differ per board.
const fn summa6_keys([plus, minus, equals, clear]: [u8; 4]) -> [(char, u8); 14] {
    let [d0, d1, d2, d3, d4, d5, d6, d7, d8, d9] = DIGIT_KEYS;
    [
        d0,
        d1,
        d2,
        d3,
        d4,
        d5,
        d6,
        d7,
        d8,
        d9,
        ('+', plus),
        ('-', minus),
        ('=', equals),
        ('C', clear),
    ]
}

/// Digits 0 to 9 send their value. The other keys differ per board.
const fn lock4_keys([enter, change, clear]: [u8; 3]) -> [(char, u8); 13] {
    let [d0, d1, d2, d3, d4, d5, d6, d7, d8, d9] = DIGIT_KEYS;
    [
        d0,
        d1,
        d2,
        d3,
        d4,
        d5,
        d6,
        d7,
        d8,
        d9,
        ('#', enter),
        ('*', change),
        ('C', clear),
    ]
}

/// Digits 0 to 9 send their value. The other keys differ per board.
const fn score5_keys([home, guest, period, clear]: [u8; 4]) -> [(char, u8); 14] {
    let [d0, d1, d2, d3, d4, d5, d6, d7, d8, d9] = DIGIT_KEYS;
    [
        d0,
        d1,
        d2,
        d3,
        d4,
        d5,
        d6,
        d7,
        d8,
        d9,
        ('H', home),
        ('G', guest),
        ('P', period),
        ('C', clear),
    ]
}
