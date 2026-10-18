use ld60_machine::{Cpu, Step};

use crate::{CalcoDevice, ChipSpec, Device, DeviceError, IsaKind, Readout, Rom};

macro_rules! each {
    ($self:expr, $device:ident => $body:expr) => {
        match $self {
            AnyDevice::Calco($device) => $body,
        }
    };
}

const TALLY2_KEYS: [(char, u8); 2] = [('+', 0x0A), ('C', 0x0D)];
const SUMMA6_KEYS: [(char, u8); 14] = [
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
    ('+', 0x0A),
    ('-', 0x0B),
    ('=', 0x0C),
    ('C', 0x0D),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeviceKind {
    /// 2-digit tally counter.
    Tally2,
    /// 6-digit adding machine.
    Summa6,
}

impl DeviceKind {
    /// RAM cells the device shows to the user.
    pub fn display_cells(self) -> std::ops::RangeInclusive<usize> {
        match self {
            Self::Tally2 => 1..=2,
            Self::Summa6 => 1..=6,
        }
    }

    /// Each key and the code the firmware reads from `KEY`.
    pub fn key_codes(self) -> &'static [(char, u8)] {
        match self {
            Self::Tally2 => &TALLY2_KEYS,
            Self::Summa6 => &SUMMA6_KEYS,
        }
    }

    pub fn isa(self) -> IsaKind {
        match self {
            Self::Tally2 | Self::Summa6 => IsaKind::Calco,
        }
    }
}

#[derive(Clone, Debug)]
pub enum AnyDevice {
    Calco(CalcoDevice),
}

impl AnyDevice {
    pub fn new(kind: DeviceKind, chip: ChipSpec, rom: Rom) -> Result<Self, DeviceError> {
        if chip.isa() != kind.isa() {
            return Err(DeviceError::ChipMismatch {
                device: kind,
                chip: chip.isa(),
            });
        }
        let ChipSpec::Calco(info) = chip;
        CalcoDevice::new(kind, info, rom).map(Self::Calco)
    }
}

impl Device for AnyDevice {
    fn kind(&self) -> DeviceKind {
        each!(self, d => d.kind())
    }

    fn isa(&self) -> IsaKind {
        each!(self, d => d.isa())
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
