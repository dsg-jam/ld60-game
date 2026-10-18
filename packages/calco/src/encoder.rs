use std::fmt;

use ld60_asm::{Encoder, Instruction, Labels, Operand};
use ld60_machine::Isa;

use crate::{Addr, Cond, Op, ROM_LEN, Revision, ports};

pub(crate) const MNEMONICS: &[&str] = &[
    "NOP", "HLT", "RET", "SWP", "CLC", "SEC", "INC", "DEC", "DA", "DS", "LD", "ST", "LDI", "CPI",
    "ADC", "SBC", "CMP", "IN", "OUT", "JMP", "JZ", "JNZ", "JC", "JNC", "CALL", "DJNZ",
];

const IN_PORTS: &[(&str, u8)] = &[("KEY", ports::KEY), ("REV", ports::REV)];
const OUT_PORTS: &[(&str, u8)] = &[("ANN", ports::ANN)];

pub(crate) struct CalcoEncoder(pub(crate) Revision);

impl CalcoEncoder {
    fn unsupported(&self, what: impl fmt::Display) -> String {
        format!("'{what}' is not supported by {}", self.0.isa().name())
    }
}

impl Encoder for CalcoEncoder {
    fn word_bits(&self) -> u8 {
        8
    }

    fn rom_len(&self) -> usize {
        ROM_LEN
    }

    fn size(&self, instruction: &Instruction) -> Result<usize, String> {
        let m = instruction.mnemonic.as_str();
        if !MNEMONICS.contains(&m) {
            return Err(format!("unknown mnemonic '{m}'"));
        }
        if !self.0.mnemonics().contains(&m) {
            return Err(self.unsupported(m));
        }
        match m {
            "LDI" | "CPI" | "JMP" | "JZ" | "JNZ" | "JC" | "JNC" | "CALL" | "DJNZ" => Ok(2),
            _ => Ok(1),
        }
    }

    fn encode(&self, instruction: &Instruction, labels: &Labels) -> Result<Vec<u16>, String> {
        let op = to_op(instruction, labels)?;
        if !self.0.supports(op) {
            return Err(self.unsupported(op));
        }
        Ok(op.encode().into_iter().map(u16::from).collect())
    }
}

fn to_op(instruction: &Instruction, labels: &Labels) -> Result<Op, String> {
    let m = instruction.mnemonic.as_str();
    let ops = instruction.operands.as_slice();
    let simple = |op| none(m, ops).map(|()| op);
    match m {
        "NOP" => simple(Op::Nop),
        "HLT" => simple(Op::Hlt),
        "RET" => simple(Op::Ret),
        "SWP" => simple(Op::Swp),
        "CLC" => simple(Op::Clc),
        "SEC" => simple(Op::Sec),
        "INC" => simple(Op::Inc),
        "DEC" => simple(Op::Dec),
        "DA" => simple(Op::Da),
        "DS" => simple(Op::Ds),
        "LD" => memory(m, ops).map(Op::Ld),
        "ST" => memory(m, ops).map(Op::St),
        "ADC" => memory(m, ops).map(Op::Adc),
        "SBC" => memory(m, ops).map(Op::Sbc),
        "CMP" => memory(m, ops).map(Op::Cmp),
        "LDI" => {
            let [reg, value] = ops else {
                return Err("LDI expects a register and a value".into());
            };
            let value = byte(labels.value(value)?)?;
            match reg {
                Operand::Name(r) if r.eq_ignore_ascii_case("A") => Ok(Op::LdiA(value)),
                Operand::Name(r) if r.eq_ignore_ascii_case("B") => Ok(Op::LdiB(value)),
                _ => Err("LDI expects register A or B".into()),
            }
        }
        "CPI" => byte(labels.value(one(m, ops)?)?).map(Op::Cpi),
        "IN" => port(one(m, ops)?, IN_PORTS).map(Op::In),
        "OUT" => port(one(m, ops)?, OUT_PORTS).map(Op::Out),
        "CALL" => byte(labels.value(one(m, ops)?)?).map(Op::Call),
        "DJNZ" => byte(labels.value(one(m, ops)?)?).map(Op::Djnz),
        _ => {
            let cond = Cond::ALL
                .into_iter()
                .find(|c| c.mnemonic() == m)
                .ok_or_else(|| format!("unknown mnemonic '{m}'"))?;
            byte(labels.value(one(m, ops)?)?).map(|t| Op::Jump(cond, t))
        }
    }
}

fn none(m: &str, ops: &[Operand]) -> Result<(), String> {
    if ops.is_empty() {
        Ok(())
    } else {
        Err(format!("{m} takes no operands"))
    }
}

fn one<'a>(m: &str, ops: &'a [Operand]) -> Result<&'a Operand, String> {
    match ops {
        [op] => Ok(op),
        _ => Err(format!("{m} expects one operand")),
    }
}

fn memory(m: &str, ops: &[Operand]) -> Result<Addr, String> {
    let expected = || format!("{m} expects Mn or [B+n]");
    match one(m, ops)? {
        Operand::Name(name) => cell(name).map(Addr::Direct).ok_or_else(expected),
        Operand::Indirect { base, offset } if base.eq_ignore_ascii_case("B") => {
            u8::try_from(*offset)
                .ok()
                .filter(|&n| n <= 0x0F)
                .map(Addr::Indexed)
                .ok_or_else(|| "offset must be 0 to 15".into())
        }
        _ => Err(expected()),
    }
}

fn cell(name: &str) -> Option<u8> {
    let digit = name.strip_prefix('M').or_else(|| name.strip_prefix('m'))?;
    let mut chars = digit.chars();
    let n = chars.next()?.to_digit(16)?;
    chars.next().is_none().then_some(n as u8)
}

fn port(operand: &Operand, names: &[(&str, u8)]) -> Result<u8, String> {
    match operand {
        Operand::Name(name) => names
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|&(_, p)| p)
            .ok_or_else(|| format!("unknown port '{name}'")),
        Operand::Number(n) => u8::try_from(*n)
            .ok()
            .filter(|&p| p <= 0x0F)
            .ok_or_else(|| "port must be 0 to 15".into()),
        Operand::Indirect { .. } => Err("expected a port".into()),
    }
}

fn byte(value: i64) -> Result<u8, String> {
    u8::try_from(value).map_err(|_| format!("value {value} is outside 0 to 255"))
}
