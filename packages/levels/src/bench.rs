use ld60_devices::{AnyDevice, Device, Fault, ProgramError, Rom};
use ld60_machine::{Assembly, Image};

use crate::{Level, LevelError};

pub fn assemble(level: &Level, source: &str) -> Result<Assembly, LevelError> {
    Ok(level.chip.isa().isa().assemble(source)?)
}

/// The ROM as the customer brings it in: the buggy firmware plus ROM damage.
pub fn initial_rom(level: &Level, buggy: &str) -> Result<Rom, LevelError> {
    let mut rom = Rom::new(level.rom.kind, assemble(level, buggy)?.image);
    for fault in &level.faults {
        if let &Fault::FadedRomBits { addr, mask } = fault {
            rom.fade(usize::from(addr), mask);
        }
    }
    Ok(rom)
}

/// Puts `image` on a blank chip if the level allows it, or burns it over `current`.
pub fn burn(level: &Level, current: &Rom, image: Image) -> Result<Rom, ProgramError> {
    if level.rom.blank_available {
        return Ok(Rom::new(level.rom.kind, image));
    }
    let mut rom = current.clone();
    rom.program(&image)?;
    Ok(rom)
}

/// The device with `rom` installed. Chip faults stay, whatever ROM is used.
pub fn device(level: &Level, rom: Rom) -> Result<AnyDevice, LevelError> {
    let mut device = AnyDevice::new(level.device, level.chip, rom)?;
    for &fault in &level.faults {
        if let Fault::StuckRamBit(_) = fault {
            device.apply_fault(fault);
        }
    }
    Ok(device)
}
