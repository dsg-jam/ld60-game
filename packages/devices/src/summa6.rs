use ld60_calco::{ChipInfo, ROM_LEN, ports};
use ld60_machine::{Bus, Cpu as _, Step};

use crate::rng::Rng;
use crate::{Device, DeviceError, DeviceKind, IsaKind, Readout, Rom};

const KEYS: [(char, u8); 14] = [
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

const DISPLAY_CELLS: std::ops::RangeInclusive<usize> = 1..=6;

/// 6-digit adding machine with a CalCo chip.
#[derive(Clone, Debug)]
pub struct Summa6 {
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

impl Summa6 {
    pub fn new(chip: ChipInfo, rom: Rom) -> Result<Self, DeviceError> {
        let actual = rom.image().len();
        if actual != ROM_LEN {
            return Err(DeviceError::RomSize {
                expected: ROM_LEN,
                actual,
            });
        }
        Ok(Self {
            cpu: ld60_calco::Cpu::new(chip),
            board: Board {
                rom,
                key: None,
                ann: 0,
            },
        })
    }
}

impl Device for Summa6 {
    fn kind(&self) -> DeviceKind {
        DeviceKind::Summa6
    }

    fn isa(&self) -> IsaKind {
        IsaKind::Calco
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
        KEYS.iter()
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
            digits: DISPLAY_CELLS.map(|cell| ram.read(cell)).collect(),
            minus: self.board.ann & ports::ANN_MINUS != 0,
            error: self.board.ann & ports::ANN_ERROR != 0,
        }
    }
}
