use ld60_machine::{Bus, Ram, Register, Step};

use crate::{Addr, ChipInfo, Cond, Errata, Erratum, Op, ports};

pub const ROM_LEN: usize = 256;
pub const RAM_LEN: usize = 16;

#[derive(Clone, Debug)]
pub struct Cpu {
    a: u8,
    b: u8,
    pc: u8,
    zero: bool,
    carry: bool,
    stack: [u8; 2],
    sp: u8,
    ram: Ram,
    chip: ChipInfo,
    errata: Errata,
}

impl Cpu {
    pub fn new(chip: ChipInfo) -> Self {
        Self {
            a: 0,
            b: 0,
            pc: 0,
            zero: false,
            carry: false,
            stack: [0; 2],
            sp: 0,
            ram: Ram::new(RAM_LEN),
            chip,
            errata: Errata::for_chip(chip),
        }
    }

    pub fn chip(&self) -> ChipInfo {
        self.chip
    }

    pub fn errata(&self) -> &Errata {
        &self.errata
    }

    pub fn a(&self) -> u8 {
        self.a
    }

    pub fn b(&self) -> u8 {
        self.b
    }

    pub fn zero(&self) -> bool {
        self.zero
    }

    pub fn carry(&self) -> bool {
        self.carry
    }

    fn fetch(&self, bus: &dyn Bus, offset: u8) -> u8 {
        bus.rom(u16::from(self.pc.wrapping_add(offset))) as u8
    }

    fn cell(&self, addr: Addr) -> usize {
        usize::from(match addr {
            Addr::Direct(n) => n,
            Addr::Indexed(n) => self.b.wrapping_add(n) & 0x0F,
        })
    }

    fn read(&self, addr: Addr) -> u8 {
        self.ram.read(self.cell(addr))
    }

    fn set_a(&mut self, value: u8) {
        self.a = value;
        self.zero = value == 0;
    }

    fn add(&mut self, value: u8) {
        let sum = u16::from(self.a) + u16::from(value) + u16::from(self.carry);
        self.carry = sum > 0xFF;
        self.set_a(sum as u8);
    }

    fn subtract(&mut self, value: u8, borrow: bool) -> u8 {
        let diff = i16::from(self.a) - i16::from(value) - i16::from(borrow);
        self.carry = diff < 0;
        self.zero = diff as u8 == 0;
        diff as u8
    }

    fn holds(&self, cond: Cond) -> bool {
        match cond {
            Cond::Always => true,
            Cond::Zero => self.zero,
            Cond::NotZero => !self.zero,
            Cond::Carry => self.carry,
            Cond::NoCarry => !self.carry,
        }
    }

    fn decimal_adjust_add(&mut self) {
        self.carry = self.a >= 10;
        let a = if self.carry { self.a - 10 } else { self.a };
        self.set_a(a);
    }

    fn decimal_adjust_sub(&mut self) {
        if self.carry {
            self.set_a(self.a.wrapping_add(10));
        } else {
            self.zero = self.a == 0;
        }
        if self.a == 0 && self.errata.has(Erratum::DsClearsCarryOnZero) {
            self.carry = false;
        }
    }
}

impl ld60_machine::Cpu for Cpu {
    fn reset(&mut self) {
        self.a = 0;
        self.b = 0;
        self.pc = 0;
        self.zero = false;
        self.carry = false;
    }

    fn step(&mut self, bus: &mut dyn Bus) -> Step {
        let op = self
            .chip
            .revision
            .decode(self.fetch(bus, 0), self.fetch(bus, 1));
        let next = self.pc.wrapping_add(op.size());
        let mut target = next;
        match op {
            Op::Nop | Op::Reserved(_) => {}
            Op::Wait => {
                if !bus.take_wake() {
                    return Step::Halted;
                }
            }
            Op::Ret => {
                self.sp ^= 1;
                target = self.stack[usize::from(self.sp)];
            }
            Op::Swp => std::mem::swap(&mut self.a, &mut self.b),
            Op::Clc => self.carry = false,
            Op::Sec => self.carry = true,
            Op::Inc => self.set_a(self.a.wrapping_add(1)),
            Op::Dec => self.set_a(self.a.wrapping_sub(1)),
            Op::Da => self.decimal_adjust_add(),
            Op::Ds => self.decimal_adjust_sub(),
            Op::Ld(addr) => self.a = self.read(addr),
            Op::St(addr) => self.ram.write(self.cell(addr), self.a),
            Op::Adc(addr) => self.add(self.read(addr)),
            Op::Sbc(addr) => {
                let value = self.subtract(self.read(addr), self.carry);
                self.a = value;
            }
            Op::Cmp(addr) => {
                self.subtract(self.read(addr), false);
            }
            Op::LdiA(value) => self.a = value,
            Op::LdiB(value) => self.b = value,
            Op::Cpi(value) => {
                self.subtract(value, false);
            }
            Op::In(port) => {
                self.a = if port == ports::REV {
                    self.chip.revision.id()
                } else {
                    bus.port_in(port)
                };
            }
            Op::Out(port) => bus.port_out(port, self.a),
            Op::Jump(cond, addr) => {
                if self.holds(cond) {
                    target = addr;
                }
            }
            Op::Call(addr) => {
                self.stack[usize::from(self.sp)] = next;
                self.sp ^= 1;
                target = addr;
            }
            Op::Djnz(addr) => {
                self.b = self.b.wrapping_sub(1);
                if self.b != 0 {
                    target = addr;
                }
            }
        }
        self.pc = target;
        Step::Executed
    }

    fn pc(&self) -> u16 {
        u16::from(self.pc)
    }

    fn registers(&self) -> Vec<Register> {
        let reg = |name, value: u8, bits| Register {
            name,
            value: u16::from(value),
            bits,
        };
        vec![
            reg("A", self.a, 8),
            reg("B", self.b, 8),
            reg("PC", self.pc, 8),
            reg("Z", u8::from(self.zero), 1),
            reg("C", u8::from(self.carry), 1),
            reg("SP", self.sp, 1),
            reg("S0", self.stack[0], 8),
            reg("S1", self.stack[1], 8),
        ]
    }

    fn ram(&self) -> &Ram {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Ram {
        &mut self.ram
    }
}
