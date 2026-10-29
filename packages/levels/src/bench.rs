use ld60_devices::{AnyDevice, Device, Fault, ProgramError, Rom};
use ld60_machine::{Assembly, Image};

use crate::{Level, LevelError};

pub fn assemble(level: &Level, source: &str) -> Result<Assembly, LevelError> {
    Ok(level.chip.isa().assemble(source)?)
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

/// The source the editor shows once `rom` is read off a device that came in
/// with `buggy`: the lines of `buggy`, except where the words on the chip
/// differ, like faded bits. Those lines show what the chip really holds,
/// disassembled, after any label they have.
pub fn dumped_source(level: &Level, buggy: &str, rom: &Image) -> Result<String, LevelError> {
    let assembly = assemble(level, buggy)?;
    let listing = level.chip.isa().disassemble(rom);
    let mut lines: Vec<String> = buggy.lines().map(str::to_owned).collect();
    for source in &assembly.lines {
        let words = source.addr..source.addr + source.len;
        let changed = words
            .clone()
            .any(|addr| assembly.image.get(addr.into()) != rom.get(addr.into()));
        let Some(line) = lines
            .get_mut(source.line.wrapping_sub(1))
            .filter(|_| changed)
        else {
            continue;
        };
        let read: Vec<&str> = listing
            .iter()
            .filter(|read| words.contains(&read.addr))
            .map(|read| read.text.as_str())
            .collect();
        let label_end = line
            .split_once(':')
            .filter(|(name, _)| {
                !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
            })
            .map_or(0, |(name, _)| name.len() + 1);
        let (label, rest) = line.split_at(label_end);
        let indent = &rest[..rest.len() - rest.trim_start().len()];
        let indent = if indent.is_empty() { " " } else { indent };
        let separator = format!("\n{}", " ".repeat(label.len() + indent.len()));
        *line = format!("{label}{indent}{}", read.join(&separator));
    }
    let mut source = lines.join("\n");
    if buggy.ends_with('\n') {
        source.push('\n');
    }
    Ok(source)
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
    let mut device = AnyDevice::new(level.board(), level.chip, rom)?;
    for &fault in &level.faults {
        if let Fault::StuckRamBit(_) = fault {
            device.apply_fault(fault);
        }
    }
    Ok(device)
}
