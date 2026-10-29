use std::fmt;

use ld60_asm::{Encoder, Instruction, Labels, Operand};
use ld60_machine::{AsmErrorKind, Isa};

use crate::{Addr, Cond, Op, ROM_LEN, Revision, ports};

pub(crate) const MNEMONICS: &[&str] = &[
    "NOP", "WAIT", "RET", "SWP", "CLC", "SEC", "INC", "DEC", "DA", "DS", "LD", "ST", "LDI", "CPI",
    "ADC", "SBC", "CMP", "IN", "OUT", "JMP", "JZ", "JNZ", "JC", "JNC", "CALL", "DJNZ",
];

/// Old names, still assembled so older sources keep working.
const ALIASES: &[(&str, &str)] = &[("HLT", "WAIT")];

pub(crate) const PORTS: &[&str] = &["KEY", "REV", "ANN"];
pub(crate) const REGISTERS: &[&str] = &[
    "A", "B", "M0", "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9", "MA", "MB", "MC", "MD",
    "ME", "MF",
];

const IN_PORTS: &[(&str, u8)] = &[("KEY", ports::KEY), ("REV", ports::REV)];
const OUT_PORTS: &[(&str, u8)] = &[("ANN", ports::ANN)];

pub(crate) struct CalcoEncoder(pub(crate) Revision);

impl CalcoEncoder {
    fn unsupported(&self, form: impl fmt::Display) -> AsmErrorKind {
        AsmErrorKind::Unsupported {
            form: form.to_string(),
            isa: self.0.isa().name(),
        }
    }
}

impl Encoder for CalcoEncoder {
    fn word_bits(&self) -> u8 {
        8
    }

    fn rom_len(&self) -> usize {
        ROM_LEN
    }

    fn size(&self, instruction: &Instruction) -> Result<usize, AsmErrorKind> {
        let m = canonical(&instruction.mnemonic);
        if !MNEMONICS.contains(&m) {
            return Err(AsmErrorKind::UnknownMnemonic { mnemonic: m.into() });
        }
        if !self.0.mnemonics().contains(&m) {
            return Err(self.unsupported(m));
        }
        match m {
            "LDI" | "CPI" | "JMP" | "JZ" | "JNZ" | "JC" | "JNC" | "CALL" | "DJNZ" => Ok(2),
            _ => Ok(1),
        }
    }

    fn encode(&self, instruction: &Instruction, labels: &Labels) -> Result<Vec<u16>, AsmErrorKind> {
        let op = to_op(instruction, labels)?;
        if !self.0.supports(op) {
            return Err(self.unsupported(op));
        }
        Ok(op.encode().into_iter().map(u16::from).collect())
    }
}

/// The current name of mnemonic `m`.
fn canonical(m: &str) -> &str {
    ALIASES
        .iter()
        .find(|(old, _)| *old == m)
        .map_or(m, |&(_, new)| new)
}

fn to_op(instruction: &Instruction, labels: &Labels) -> Result<Op, AsmErrorKind> {
    let m = canonical(&instruction.mnemonic);
    let ops = instruction.operands.as_slice();
    let simple = |op| none(m, ops).map(|()| op);
    match m {
        "NOP" => simple(Op::Nop),
        "WAIT" => simple(Op::Wait),
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
                return Err(AsmErrorKind::ExpectsRegisterAndValue { mnemonic: m.into() });
            };
            let value = byte(labels.value(value)?)?;
            match reg {
                Operand::Name(r) if r.eq_ignore_ascii_case("A") => Ok(Op::LdiA(value)),
                Operand::Name(r) if r.eq_ignore_ascii_case("B") => Ok(Op::LdiB(value)),
                _ => Err(AsmErrorKind::ExpectsAOrB { mnemonic: m.into() }),
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
                .ok_or_else(|| AsmErrorKind::UnknownMnemonic { mnemonic: m.into() })?;
            byte(labels.value(one(m, ops)?)?).map(|t| Op::Jump(cond, t))
        }
    }
}

fn none(m: &str, ops: &[Operand]) -> Result<(), AsmErrorKind> {
    if ops.is_empty() {
        Ok(())
    } else {
        Err(AsmErrorKind::TakesNoOperands { mnemonic: m.into() })
    }
}

fn one<'a>(m: &str, ops: &'a [Operand]) -> Result<&'a Operand, AsmErrorKind> {
    match ops {
        [op] => Ok(op),
        _ => Err(AsmErrorKind::ExpectsOneOperand { mnemonic: m.into() }),
    }
}

fn memory(m: &str, ops: &[Operand]) -> Result<Addr, AsmErrorKind> {
    let expected = || AsmErrorKind::ExpectsMemory { mnemonic: m.into() };
    match one(m, ops)? {
        Operand::Name(name) => cell(name).map(Addr::Direct).ok_or_else(expected),
        Operand::Indirect { base, offset } if base.eq_ignore_ascii_case("B") => {
            u8::try_from(*offset)
                .ok()
                .filter(|&n| n <= 0x0F)
                .map(Addr::Indexed)
                .ok_or(AsmErrorKind::OffsetRange { offset: *offset })
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

fn port(operand: &Operand, names: &[(&str, u8)]) -> Result<u8, AsmErrorKind> {
    match operand {
        Operand::Name(name) => names
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|&(_, p)| p)
            .ok_or_else(|| AsmErrorKind::UnknownPort { name: name.clone() }),
        Operand::Number(n) => u8::try_from(*n)
            .ok()
            .filter(|&p| p <= 0x0F)
            .ok_or(AsmErrorKind::PortRange { port: *n }),
        Operand::Indirect { .. } => Err(AsmErrorKind::ExpectsPort),
    }
}

fn byte(value: i64) -> Result<u8, AsmErrorKind> {
    u8::try_from(value).map_err(|_| AsmErrorKind::ByteRange { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_every_port_and_cell() {
        for (name, _) in IN_PORTS.iter().chain(OUT_PORTS) {
            assert!(PORTS.contains(name), "{name}");
        }
        let cells = REGISTERS.iter().filter(|name| cell(name).is_some()).count();
        assert_eq!(cells, 16);
    }
}
