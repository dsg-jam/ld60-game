use std::collections::BTreeSet;

use ld60_devices::{AnyDevice, Device, MAX_STEPS, Rom};
use ld60_levels::{Level, LevelSource, Report, TEST_SEEDS, assemble, burn, device, initial_rom};
use ld60_machine::{Assembly, Step};

/// A level on the bench: the device, the firmware it runs and the breakpoints.
pub struct Session {
    pub level: Level,
    pub device: AnyDevice,
    pub breakpoints: BTreeSet<u16>,
    pub seed: u64,
    source: String,
    assembly: Assembly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Halted,
    Breakpoint(u16),
    Timeout,
}

impl Session {
    pub fn new(source: &LevelSource, fixed: bool, seed: u64) -> crate::Result<Self> {
        let level = source.load()?;
        let initial = initial_rom(&level, source.buggy)?;
        let firmware = if fixed { source.fixed } else { source.buggy };
        let assembly = assemble(&level, firmware)?;
        let rom = if fixed {
            burn(&level, &initial, assembly.image.clone())?
        } else {
            initial
        };
        let mut session = Self {
            device: device(&level, rom)?,
            level,
            breakpoints: BTreeSet::new(),
            seed,
            source: firmware.to_string(),
            assembly,
        };
        session.power_on(seed);
        Ok(session)
    }

    pub fn power_on(&mut self, seed: u64) -> Stop {
        self.seed = seed;
        self.device.power_on(seed);
        self.run()
    }

    pub fn run(&mut self) -> Stop {
        for i in 0..MAX_STEPS {
            let pc = self.device.cpu().pc();
            if i > 0 && self.breakpoints.contains(&pc) {
                return Stop::Breakpoint(pc);
            }
            if self.device.step() == Step::Halted {
                return Stop::Halted;
            }
        }
        Stop::Timeout
    }

    pub fn step(&mut self) -> Step {
        self.device.step()
    }

    /// Presses each key and runs until the firmware waits again.
    pub fn press(&mut self, keys: &str) -> crate::Result<Stop> {
        let mut stop = Stop::Halted;
        for key in keys.chars().filter(|c| !c.is_whitespace()) {
            self.device.press_key(key)?;
            stop = self.run();
            if stop != Stop::Halted {
                break;
            }
        }
        Ok(stop)
    }

    /// Assembles `source` and puts it on the chip, then powers on.
    pub fn burn(&mut self, source: String) -> crate::Result<Stop> {
        let assembly = assemble(&self.level, &source)?;
        let rom: Rom = burn(&self.level, self.device.rom(), assembly.image.clone())?;
        *self.device.rom_mut() = rom;
        self.source = source;
        self.assembly = assembly;
        Ok(self.power_on(self.seed))
    }

    pub fn test(&self) -> crate::Result<Report> {
        Ok(ld60_levels::run(
            &self.level,
            self.device.rom(),
            TEST_SEEDS,
        )?)
    }

    pub fn label(&self, name: &str) -> Option<u16> {
        self.assembly.labels.get(name).copied()
    }

    /// The source line that produced the word at `addr`, if any.
    pub fn source_line(&self, addr: u16) -> Option<(usize, &str)> {
        let line = self.assembly.line_at(addr)?;
        let text = self.source.lines().nth(line - 1)?;
        Some((line, text.trim()))
    }
}
