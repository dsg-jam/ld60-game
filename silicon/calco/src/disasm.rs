use std::fmt;

use ld60_machine::{DisasmLine, Image};

use crate::{Addr, Op, Revision, ports};

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Op::Nop => f.write_str("NOP"),
            Op::Wait => f.write_str("WAIT"),
            Op::Ret => f.write_str("RET"),
            Op::Swp => f.write_str("SWP"),
            Op::Clc => f.write_str("CLC"),
            Op::Sec => f.write_str("SEC"),
            Op::Inc => f.write_str("INC"),
            Op::Dec => f.write_str("DEC"),
            Op::Da => f.write_str("DA"),
            Op::Ds => f.write_str("DS"),
            Op::Ld(a) => write!(f, "LD {a}"),
            Op::St(a) => write!(f, "ST {a}"),
            Op::Adc(a) => write!(f, "ADC {a}"),
            Op::Sbc(a) => write!(f, "SBC {a}"),
            Op::Cmp(a) => write!(f, "CMP {a}"),
            Op::LdiA(v) => write!(f, "LDI A, {v}"),
            Op::LdiB(v) => write!(f, "LDI B, {v}"),
            Op::Cpi(v) => write!(f, "CPI {v}"),
            Op::In(ports::KEY) => f.write_str("IN KEY"),
            Op::In(ports::REV) => f.write_str("IN REV"),
            Op::In(p) => write!(f, "IN {p}"),
            Op::Out(ports::ANN) => f.write_str("OUT ANN"),
            Op::Out(p) => write!(f, "OUT {p}"),
            Op::Jump(c, t) => write!(f, "{} 0x{t:02X}", c.mnemonic()),
            Op::Call(t) => write!(f, "CALL 0x{t:02X}"),
            Op::Djnz(t) => write!(f, "DJNZ 0x{t:02X}"),
            Op::Reserved(b) => write!(f, ".db 0x{b:02X}"),
        }
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Addr::Direct(n) => write!(f, "M{n:X}"),
            Addr::Indexed(0) => f.write_str("[B]"),
            Addr::Indexed(n) => write!(f, "[B+{n}]"),
        }
    }
}

pub(crate) fn disassemble(image: &Image, revision: Revision) -> Vec<DisasmLine> {
    let mut lines = Vec::new();
    let mut addr = 0;
    while addr < image.len() {
        let byte = |a: usize| image.get(a % image.len()) as u8;
        let op = match revision.decode(byte(addr), byte(addr + 1)) {
            op if addr + usize::from(op.size()) > image.len() => Op::Reserved(byte(addr)),
            op => op,
        };
        let size = usize::from(op.size());
        lines.push(DisasmLine {
            addr: addr as u16,
            words: image.words()[addr..addr + size].to_vec(),
            text: op.to_string(),
        });
        addr += size;
    }
    lines
}
