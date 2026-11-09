use ld60_machine::Image;

use crate::{Op, Revision};

/// The core of a Tick chip: a program counter that goes back to word 0
/// after the last word of each page, and whether it sleeps until the next
/// tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cpu {
    revision: Revision,
    pc: u8,
    waiting: bool,
}

/// What one step did outside the chip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Nothing outside the chip changed.
    None,
    /// The hand moves one mark on.
    Step,
    /// The hand goes back to the top.
    Home,
    /// The chip is asleep until the next tick and did nothing.
    Asleep,
}

impl Cpu {
    pub fn new(revision: Revision) -> Self {
        Self {
            revision,
            pc: 0,
            waiting: false,
        }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// The address of the next instruction.
    pub fn pc(&self) -> u8 {
        self.pc
    }

    /// Sets the address of the next instruction, wrapping at the end of ROM.
    pub fn set_pc(&mut self, pc: u8) {
        self.pc = pc % self.len();
    }

    pub fn waiting(&self) -> bool {
        self.waiting
    }

    pub fn reset(&mut self) {
        self.pc = 0;
        self.waiting = false;
    }

    /// The once-a-second tick from the crystal wakes a waiting chip.
    pub fn tick(&mut self) {
        self.waiting = false;
    }

    /// A key press sends the program to [`Revision::KEY_VECTOR`] and wakes
    /// it. Chips without a key ignore it.
    pub fn press(&mut self) {
        if self.revision.has_key() {
            self.pc = Revision::KEY_VECTOR;
            self.waiting = false;
        }
    }

    /// Runs the instruction at the program counter from `rom`, unless the
    /// chip is asleep.
    pub fn step(&mut self, rom: &Image) -> Effect {
        if self.waiting {
            return Effect::Asleep;
        }
        let word = rom.get(usize::from(self.pc) % rom.len());
        let next = self.pc + 1;
        self.set_pc(if next.is_multiple_of(Revision::PAGE) { 0 } else { next });
        match self.revision.decode(word as u8) {
            Op::Step => Effect::Step,
            Op::Home => Effect::Home,
            Op::Wait => {
                self.waiting = true;
                Effect::None
            }
            Op::Nop | Op::Reserved(_) => Effect::None,
        }
    }

    fn len(&self) -> u8 {
        self.revision.rom_len() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WORD_BITS;

    fn rom(revision: Revision, words: &[u8]) -> Image {
        let mut image = Image::blank(revision.rom_len(), WORD_BITS);
        for (addr, &word) in words.iter().enumerate() {
            image.set(addr, u16::from(word));
        }
        image
    }

    #[test]
    fn an_erased_rom_does_nothing_and_wraps() {
        let mut cpu = Cpu::new(Revision::A);
        let rom = rom(Revision::A, &[]);
        let pcs: Vec<_> = (0..5)
            .map(|_| {
                assert_eq!(cpu.step(&rom), Effect::None);
                cpu.pc()
            })
            .collect();
        assert_eq!(pcs, [1, 2, 3, 0, 1]);
    }

    #[test]
    fn step_then_wait_sleeps_until_the_tick() {
        let mut cpu = Cpu::new(Revision::A);
        let rom = rom(Revision::A, &[Op::STEP, Op::WAIT]);
        assert_eq!(cpu.step(&rom), Effect::Step);
        assert_eq!(cpu.step(&rom), Effect::None);
        assert_eq!(cpu.step(&rom), Effect::Asleep);
        cpu.tick();
        assert_eq!(cpu.step(&rom), Effect::None);
        assert_eq!(cpu.step(&rom), Effect::None);
        assert_eq!(cpu.step(&rom), Effect::Step);
    }

    #[test]
    fn home_and_the_key_need_revision_b() {
        let words = [Op::HOME];
        let mut a = Cpu::new(Revision::A);
        assert_eq!(a.step(&rom(Revision::A, &words)), Effect::None);
        a.press();
        assert_eq!(a.pc(), 1);

        let mut b = Cpu::new(Revision::B);
        assert_eq!(b.step(&rom(Revision::B, &words)), Effect::Home);
        b.set_pc(3);
        b.step(&rom(Revision::B, &[]));
        assert_eq!(b.pc(), 0, "the first page wraps on itself");
        b.set_pc(7);
        b.step(&rom(Revision::B, &[]));
        assert_eq!(b.pc(), 0, "the key page goes back to the first");
        let sleepy = rom(Revision::B, &[Op::NOP, Op::WAIT]);
        b.step(&sleepy);
        b.step(&sleepy);
        assert!(b.waiting());
        b.press();
        assert_eq!((b.pc(), b.waiting()), (Revision::KEY_VECTOR, false));
    }
}
