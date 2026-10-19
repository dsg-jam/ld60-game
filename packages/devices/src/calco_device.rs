use ld60_calco::{ChipInfo, ROM_LEN, ports};
use ld60_machine::{Bus, Cpu as _, Step};

use crate::rng::Rng;
use crate::{Annunciator, Board, ChipSpec, Device, DeviceError, DeviceKind, Readout, Rom};

/// A device built around a CalCo chip. The board decides the keys and the
/// display.
#[derive(Clone, Debug)]
pub struct CalcoDevice {
    board: &'static Board,
    cpu: ld60_calco::Cpu,
    circuit: Circuit,
}

/// What the chip sees on its pins.
#[derive(Clone, Debug)]
struct Circuit {
    rom: Rom,
    key: Option<u8>,
    ann: u8,
}

impl Bus for Circuit {
    fn rom(&self, addr: u16) -> u16 {
        self.rom.image().get(usize::from(addr) % ROM_LEN)
    }

    fn port_in(&mut self, port: u8) -> u8 {
        match port {
            ports::KEY => self.key.take().unwrap_or(ports::NO_KEY),
            _ => 0xFF,
        }
    }

    fn port_out(&mut self, port: u8, value: u8) {
        if port == ports::ANN {
            self.ann = value;
        }
    }

    fn wake_requested(&self) -> bool {
        self.key.is_some()
    }
}

impl CalcoDevice {
    pub fn new(board: &'static Board, chip: ChipInfo, rom: Rom) -> Result<Self, DeviceError> {
        let actual = rom.image().len();
        if actual != ROM_LEN {
            return Err(DeviceError::RomSize {
                expected: ROM_LEN,
                actual,
            });
        }
        Ok(Self {
            board,
            cpu: ld60_calco::Cpu::new(chip),
            circuit: Circuit {
                rom,
                key: None,
                ann: 0,
            },
        })
    }

    fn lit(&self, annunciator: Annunciator) -> bool {
        self.board
            .ann_bit(annunciator)
            .is_some_and(|bit| self.circuit.ann >> bit & 1 != 0)
    }
}

impl Device for CalcoDevice {
    fn board(&self) -> &'static Board {
        self.board
    }

    fn kind(&self) -> DeviceKind {
        self.board.kind
    }

    fn chip(&self) -> ChipSpec {
        ChipSpec::Calco(self.cpu.chip())
    }

    fn cpu(&self) -> &dyn ld60_machine::Cpu {
        &self.cpu
    }

    fn cpu_mut(&mut self) -> &mut dyn ld60_machine::Cpu {
        &mut self.cpu
    }

    fn rom(&self) -> &Rom {
        &self.circuit.rom
    }

    fn rom_mut(&mut self) -> &mut Rom {
        &mut self.circuit.rom
    }

    fn power_on(&mut self, seed: u64) {
        let mut rng = Rng::new(seed);
        self.cpu.ram_mut().fill(|| rng.next_u8());
        self.cpu.reset();
        self.circuit.key = None;
        self.circuit.ann = 0;
    }

    fn key_code(&self, key: char) -> Option<u8> {
        self.board.key_code(key)
    }

    fn press(&mut self, code: u8) {
        self.circuit.key = Some(code);
    }

    fn step(&mut self) -> Step {
        self.cpu.step(&mut self.circuit)
    }

    fn readout(&self) -> Readout {
        let ram = self.cpu.ram();
        Readout {
            digits: self
                .board
                .display_cells()
                .map(|cell| ram.read(cell))
                .collect(),
            minus: self.lit(Annunciator::Minus),
            error: self.lit(Annunciator::Error),
            open: self.lit(Annunciator::Open),
            shut: self.lit(Annunciator::Shut),
        }
    }
}
