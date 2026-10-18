#![allow(dead_code)]

use ld60_calco::{Calco, ChipInfo, Cpu, DateCode, Revision};
use ld60_machine::{Bus, Cpu as _, Image, Isa, Step};

pub(crate) const LATE: ChipInfo = ChipInfo {
    revision: Revision::A,
    date_code: DateCode::new(76, 30),
};

pub(crate) const EARLY: ChipInfo = ChipInfo {
    revision: Revision::A,
    date_code: DateCode::new(76, 14),
};

pub(crate) struct TestBus {
    pub rom: Image,
    pub key: Option<u8>,
}

impl Bus for TestBus {
    fn rom(&self, addr: u16) -> u16 {
        self.rom.get(usize::from(addr))
    }

    fn port_in(&mut self, _port: u8) -> u8 {
        self.key.take().unwrap_or(0xFF)
    }

    fn port_out(&mut self, _port: u8, _value: u8) {}

    fn wake_requested(&self) -> bool {
        self.key.is_some()
    }
}

pub(crate) fn assemble(source: &str) -> Image {
    match Calco.assemble(source) {
        Ok(asm) => asm.image,
        Err(errors) => panic!("{errors:?}"),
    }
}

/// Runs until `HLT` halts or `max_steps` is reached. Returns the step count.
pub(crate) fn run(source: &str, chip: ChipInfo, max_steps: usize) -> (Cpu, usize) {
    let mut bus = TestBus {
        rom: assemble(source),
        key: None,
    };
    let mut cpu = Cpu::new(chip);
    cpu.reset();
    for steps in 0..max_steps {
        if cpu.step(&mut bus) == Step::Halted {
            return (cpu, steps);
        }
    }
    (cpu, max_steps)
}
