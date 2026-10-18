use ld60_calco::{ChipInfo, ROM_LEN, ports};
use ld60_machine::{Bus, Cpu as _, Step};

use crate::rng::Rng;
use crate::{ChipSpec, Device, DeviceError, DeviceKind, Readout, Rom};

/// A device built around a CalCo chip. The kind decides the keys and the
/// display.
#[derive(Clone, Debug)]
pub struct CalcoDevice {
    kind: DeviceKind,
    cpu: ld60_calco::Cpu,
    board: Board,
}

#[derive(Clone, Debug)]
struct Board {
    rom: Rom,
    key: Option<u8>,
    ann: u8,
}

impl Bus for Board {
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
    pub fn new(kind: DeviceKind, chip: ChipInfo, rom: Rom) -> Result<Self, DeviceError> {
        let actual = rom.image().len();
        if actual != ROM_LEN {
            return Err(DeviceError::RomSize {
                expected: ROM_LEN,
                actual,
            });
        }
        Ok(Self {
            kind,
            cpu: ld60_calco::Cpu::new(chip),
            board: Board {
                rom,
                key: None,
                ann: 0,
            },
        })
    }
}

impl Device for CalcoDevice {
    fn kind(&self) -> DeviceKind {
        self.kind
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
        &self.board.rom
    }

    fn rom_mut(&mut self) -> &mut Rom {
        &mut self.board.rom
    }

    fn power_on(&mut self, seed: u64) {
        let mut rng = Rng::new(seed);
        self.cpu.ram_mut().fill(|| rng.next_u8());
        self.cpu.reset();
        self.board.key = None;
        self.board.ann = 0;
    }

    fn key_code(&self, key: char) -> Option<u8> {
        self.kind
            .key_codes()
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&key))
            .map(|&(_, code)| code)
    }

    fn press(&mut self, code: u8) {
        self.board.key = Some(code);
    }

    fn step(&mut self) -> Step {
        self.cpu.step(&mut self.board)
    }

    fn readout(&self) -> Readout {
        let ram = self.cpu.ram();
        Readout {
            digits: self
                .kind
                .display_cells()
                .map(|cell| ram.read(cell))
                .collect(),
            minus: self.board.ann & ports::ANN_MINUS != 0,
            error: self.board.ann & ports::ANN_ERROR != 0,
        }
    }
}
