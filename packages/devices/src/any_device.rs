use ld60_machine::{Cpu, Step};

use crate::{ChipSpec, Device, DeviceError, IsaKind, Readout, Rom, Summa6};

macro_rules! each {
    ($self:expr, $device:ident => $body:expr) => {
        match $self {
            AnyDevice::Summa6($device) => $body,
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeviceKind {
    Summa6,
}

impl DeviceKind {
    /// RAM cells the device shows to the user.
    pub fn display_cells(self) -> std::ops::RangeInclusive<usize> {
        match self {
            Self::Summa6 => crate::summa6::DISPLAY_CELLS,
        }
    }

    pub fn isa(self) -> IsaKind {
        match self {
            Self::Summa6 => IsaKind::Calco,
        }
    }
}

#[derive(Clone, Debug)]
pub enum AnyDevice {
    Summa6(Summa6),
}

impl AnyDevice {
    pub fn new(kind: DeviceKind, chip: ChipSpec, rom: Rom) -> Result<Self, DeviceError> {
        if chip.isa() != kind.isa() {
            return Err(DeviceError::ChipMismatch {
                device: kind,
                chip: chip.isa(),
            });
        }
        match kind {
            DeviceKind::Summa6 => {
                let ChipSpec::Calco(info) = chip;
                Summa6::new(info, rom).map(Self::Summa6)
            }
        }
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
