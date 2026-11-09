mod wanted;

use ld60_machine::Image;
use ld60_tick::{Cpu, Effect, Revision, WORD_BITS};

pub use self::wanted::{Wanted, WatchComplaint};
use crate::{DeviceError, Rom, RomKind};

/// Marks around the dial.
pub const MARKS: u8 = 12;

/// A bare watch movement: a Tick chip with its EPROM, driving one hand
/// around a dial.
#[derive(Clone, Debug)]
pub struct Movement {
    cpu: Cpu,
    rom: Rom,
    /// The mark the hand points at. 0 is the top.
    hand: u8,
}

impl Movement {
    /// A movement with an erased EPROM.
    pub fn blank(revision: Revision) -> Self {
        let rom = Rom::new(RomKind::Eprom, Image::blank(revision.rom_len(), WORD_BITS));
        Self {
            cpu: Cpu::new(revision),
            rom,
            hand: 0,
        }
    }

    pub fn new(revision: Revision, rom: Rom) -> Result<Self, DeviceError> {
        let expected = revision.rom_len();
        let actual = rom.image().len();
        if actual != expected {
            return Err(DeviceError::RomSize { expected, actual });
        }
        Ok(Self {
            rom,
            ..Self::blank(revision)
        })
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    pub fn rom(&self) -> &Rom {
        &self.rom
    }

    pub fn rom_mut(&mut self) -> &mut Rom {
        &mut self.rom
    }

    pub fn hand(&self) -> u8 {
        self.hand
    }

    /// Runs one instruction and moves the hand if it says so.
    pub fn step(&mut self) -> Effect {
        let effect = self.cpu.step(self.rom.image());
        match effect {
            Effect::Step => self.hand = (self.hand + 1) % MARKS,
            Effect::Home => self.hand = 0,
            Effect::None | Effect::Asleep => {}
        }
        effect
    }

    /// Starts from the top with the chip reset, as when a battery goes in.
    pub fn power_on(&mut self) {
        self.cpu.reset();
        self.hand = 0;
    }

    pub fn tick(&mut self) {
        self.cpu.tick();
    }

    pub fn press(&mut self) {
        self.cpu.press();
    }
}

#[cfg(test)]
mod tests {
    use ld60_tick::Op;

    use super::*;

    #[test]
    fn the_first_job_steps_once_a_tick() {
        let mut movement = Movement::blank(Revision::A);
        for (addr, word) in [Op::STEP, Op::WAIT].into_iter().enumerate() {
            movement
                .rom_mut()
                .program_word(addr, u16::from(word))
                .unwrap();
        }
        for second in 1..=13 {
            while movement.step() != Effect::Asleep {}
            assert_eq!(movement.hand(), second % MARKS);
            movement.tick();
        }
    }

    #[test]
    fn a_programmed_word_needs_an_erase_to_change() {
        let mut movement = Movement::blank(Revision::A);
        let rom = movement.rom_mut();
        rom.program_word(0, u16::from(Op::STEP)).unwrap();
        assert!(rom.program_word(0, u16::from(Op::WAIT)).is_err());
        rom.erase().unwrap();
        rom.program_word(0, u16::from(Op::WAIT)).unwrap();
    }
}
