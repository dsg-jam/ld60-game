//! Tables for the printed datasheet. They match `docs/isa/calco.md`.
//!
//! Each entry names an [`Op`], so a datasheet for one chip can show only
//! what [`Revision::supports`](crate::Revision::supports). The game words
//! what each row does for players in its `datasheet.ftl`, named after the
//! row's hex code.

use self::FlagEffect::{Cleared, FromResult, Kept, Set};
use self::Group::{Jumps, Keys, Lights, Loops, Math, Numbers, Routines};
use crate::{Addr, Cond, Op};

const DIRECT: Addr = Addr::Direct(0);
const INDEXED: Addr = Addr::Indexed(0);

pub const INSTRUCTIONS: &[InstructionDoc] = &[
    InstructionDoc {
        op: Op::Nop,
        hex: "00",
        syntax: "NOP",
        z: Kept,
        c: Kept,
        group: None,
        example: "NOP",
    },
    InstructionDoc {
        op: Op::Wait,
        hex: "01",
        syntax: "WAIT",
        z: Kept,
        c: Kept,
        group: Some(Keys),
        example: "WAIT",
    },
    InstructionDoc {
        op: Op::Ret,
        hex: "02",
        syntax: "RET",
        z: Kept,
        c: Kept,
        group: Some(Routines),
        example: "RET",
    },
    InstructionDoc {
        op: Op::Swp,
        hex: "03",
        syntax: "SWP",
        z: Kept,
        c: Kept,
        group: Some(Numbers),
        example: "SWP",
    },
    InstructionDoc {
        op: Op::Clc,
        hex: "04",
        syntax: "CLC",
        z: Kept,
        c: Cleared,
        group: Some(Math),
        example: "CLC",
    },
    InstructionDoc {
        op: Op::Sec,
        hex: "05",
        syntax: "SEC",
        z: Kept,
        c: Set,
        group: Some(Math),
        example: "SEC",
    },
    InstructionDoc {
        op: Op::Inc,
        hex: "06",
        syntax: "INC",
        z: FromResult,
        c: Kept,
        group: Some(Numbers),
        example: "INC",
    },
    InstructionDoc {
        op: Op::Dec,
        hex: "07",
        syntax: "DEC",
        z: FromResult,
        c: Kept,
        group: Some(Numbers),
        example: "DEC",
    },
    InstructionDoc {
        op: Op::Da,
        hex: "08",
        syntax: "DA",
        z: FromResult,
        c: FromResult,
        group: Some(Math),
        example: "DA",
    },
    InstructionDoc {
        op: Op::Ds,
        hex: "09",
        syntax: "DS",
        z: FromResult,
        c: Kept,
        group: Some(Math),
        example: "DS",
    },
    InstructionDoc {
        op: Op::Ld(DIRECT),
        hex: "1n",
        syntax: "LD Mn",
        z: Kept,
        c: Kept,
        group: Some(Numbers),
        example: "LD M2",
    },
    InstructionDoc {
        op: Op::St(DIRECT),
        hex: "2n",
        syntax: "ST Mn",
        z: Kept,
        c: Kept,
        group: Some(Numbers),
        example: "ST M2",
    },
    InstructionDoc {
        op: Op::LdiA(0),
        hex: "30 ii",
        syntax: "LDI A, ii",
        z: Kept,
        c: Kept,
        group: Some(Numbers),
        example: "LDI A, 5",
    },
    InstructionDoc {
        op: Op::LdiB(0),
        hex: "31 ii",
        syntax: "LDI B, ii",
        z: Kept,
        c: Kept,
        group: Some(Loops),
        example: "LDI B, 4",
    },
    InstructionDoc {
        op: Op::Cpi(0),
        hex: "32 ii",
        syntax: "CPI ii",
        z: FromResult,
        c: FromResult,
        group: Some(Jumps),
        example: "CPI 10",
    },
    InstructionDoc {
        op: Op::Adc(DIRECT),
        hex: "4n",
        syntax: "ADC Mn",
        z: FromResult,
        c: FromResult,
        group: Some(Math),
        example: "ADC M2",
    },
    InstructionDoc {
        op: Op::Sbc(DIRECT),
        hex: "5n",
        syntax: "SBC Mn",
        z: FromResult,
        c: FromResult,
        group: Some(Math),
        example: "SBC M2",
    },
    InstructionDoc {
        op: Op::Cmp(DIRECT),
        hex: "6n",
        syntax: "CMP Mn",
        z: FromResult,
        c: FromResult,
        group: Some(Jumps),
        example: "CMP M2",
    },
    InstructionDoc {
        op: Op::In(0),
        hex: "7p",
        syntax: "IN p",
        z: Kept,
        c: Kept,
        group: Some(Keys),
        example: "IN KEY",
    },
    InstructionDoc {
        op: Op::Out(0),
        hex: "8p",
        syntax: "OUT p",
        z: Kept,
        c: Kept,
        group: Some(Lights),
        example: "OUT ANN",
    },
    InstructionDoc {
        op: Op::Ld(INDEXED),
        hex: "9n",
        syntax: "LD [B+n]",
        z: Kept,
        c: Kept,
        group: Some(Loops),
        example: "LD [B+2]",
    },
    InstructionDoc {
        op: Op::St(INDEXED),
        hex: "An",
        syntax: "ST [B+n]",
        z: Kept,
        c: Kept,
        group: Some(Loops),
        example: "ST [B+2]",
    },
    InstructionDoc {
        op: Op::Jump(Cond::Always, 0),
        hex: "B0 aa",
        syntax: "JMP aa",
        z: Kept,
        c: Kept,
        group: Some(Jumps),
        example: "JMP count",
    },
    InstructionDoc {
        op: Op::Jump(Cond::Zero, 0),
        hex: "B1 aa",
        syntax: "JZ aa",
        z: Kept,
        c: Kept,
        group: Some(Jumps),
        example: "JZ done",
    },
    InstructionDoc {
        op: Op::Jump(Cond::NotZero, 0),
        hex: "B2 aa",
        syntax: "JNZ aa",
        z: Kept,
        c: Kept,
        group: Some(Jumps),
        example: "JNZ done",
    },
    InstructionDoc {
        op: Op::Jump(Cond::Carry, 0),
        hex: "B3 aa",
        syntax: "JC aa",
        z: Kept,
        c: Kept,
        group: Some(Jumps),
        example: "JC done",
    },
    InstructionDoc {
        op: Op::Jump(Cond::NoCarry, 0),
        hex: "B4 aa",
        syntax: "JNC aa",
        z: Kept,
        c: Kept,
        group: Some(Jumps),
        example: "JNC done",
    },
    InstructionDoc {
        op: Op::Adc(INDEXED),
        hex: "Cn",
        syntax: "ADC [B+n]",
        z: FromResult,
        c: FromResult,
        group: Some(Loops),
        example: "ADC [B+2]",
    },
    InstructionDoc {
        op: Op::Sbc(INDEXED),
        hex: "Dn",
        syntax: "SBC [B+n]",
        z: FromResult,
        c: FromResult,
        group: Some(Loops),
        example: "SBC [B+2]",
    },
    InstructionDoc {
        op: Op::Cmp(INDEXED),
        hex: "En",
        syntax: "CMP [B+n]",
        z: FromResult,
        c: FromResult,
        group: Some(Loops),
        example: "CMP [B+2]",
    },
    InstructionDoc {
        op: Op::Call(0),
        hex: "F0 aa",
        syntax: "CALL aa",
        z: Kept,
        c: Kept,
        group: Some(Routines),
        example: "CALL show",
    },
    InstructionDoc {
        op: Op::Djnz(0),
        hex: "F1 aa",
        syntax: "DJNZ aa",
        z: Kept,
        c: Kept,
        group: Some(Loops),
        example: "DJNZ next",
    },
];

pub const PORTS: &[PortDoc] = &[
    PortDoc {
        port: 0x0,
        name: "KEY",
        output: false,
    },
    PortDoc {
        port: 0xF,
        name: "REV",
        output: false,
    },
    PortDoc {
        port: 0x0,
        name: "ANN",
        output: true,
    },
];

/// The instructions with a note under them in the datasheet.
pub const NOTES: &[Op] = &[
    Op::Adc(DIRECT),
    Op::Sbc(DIRECT),
    Op::Djnz(0),
    Op::Call(0),
    Op::Ld(INDEXED),
    Op::Ds,
];

/// What an instruction is for. The datasheet lists the instructions of a
/// group together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum Group {
    Numbers,
    Keys,
    Jumps,
    Math,
    Routines,
    Loops,
    Lights,
}

/// How an instruction changes a flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagEffect {
    Kept,
    FromResult,
    Cleared,
    Set,
}

impl FlagEffect {
    pub fn symbol(self) -> char {
        match self {
            Self::Kept => '-',
            Self::FromResult => '*',
            Self::Cleared => '0',
            Self::Set => '1',
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InstructionDoc {
    /// One of the ops the row describes.
    pub op: Op,
    pub hex: &'static str,
    pub syntax: &'static str,
    pub z: FlagEffect,
    pub c: FlagEffect,
    /// Where the instruction goes in the plain list. `None` keeps it to
    /// the table.
    pub group: Option<Group>,
    /// The syntax with real operands, like `LDI A, 5`.
    pub example: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct PortDoc {
    pub port: u8,
    pub name: &'static str,
    pub output: bool,
}

impl PortDoc {
    /// The instruction that uses the port.
    pub fn op(&self) -> Op {
        if self.output {
            Op::Out(self.port)
        } else {
            Op::In(self.port)
        }
    }
}

impl Cond {
    /// The jump in datasheet syntax, like `JZ aa`.
    pub fn form(self) -> &'static str {
        match self {
            Self::Always => "JMP aa",
            Self::Zero => "JZ aa",
            Self::NotZero => "JNZ aa",
            Self::Carry => "JC aa",
            Self::NoCarry => "JNC aa",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Revision;

    #[test]
    fn each_revision_documents_exactly_its_mnemonics() {
        for revision in Revision::ALL {
            let mut documented: Vec<_> = INSTRUCTIONS
                .iter()
                .filter(|doc| revision.supports(doc.op))
                .filter_map(|doc| doc.syntax.split_whitespace().next())
                .collect();
            documented.sort_unstable();
            documented.dedup();
            let mut mnemonics = revision.mnemonics().to_vec();
            mnemonics.sort_unstable();
            assert_eq!(documented, mnemonics, "Rev {revision}");
        }
    }

    #[test]
    fn rows_match_their_op() {
        for doc in INSTRUCTIONS {
            let hex = u8::from_str_radix(&doc.hex[..2].replace(['n', 'p'], "0"), 16);
            assert_eq!(hex.map(|first| Op::decode(first, 0)), Ok(doc.op), "{doc:?}");
        }
    }

    #[test]
    fn jump_rows_match_their_condition() {
        for cond in Cond::ALL {
            let rows: Vec<_> = INSTRUCTIONS
                .iter()
                .filter(|doc| doc.op == Op::Jump(cond, 0))
                .collect();
            assert_eq!(rows.len(), 1, "{cond:?}");
            assert_eq!(rows[0].syntax, cond.form());
            assert_eq!(rows[0].hex, format!("B{:X} aa", cond.code()));
        }
    }

    #[test]
    fn rev_a_rows_are_the_rev_a_ops() {
        let rows: Vec<_> = INSTRUCTIONS
            .iter()
            .filter(|doc| Revision::A.supports(doc.op))
            .map(|doc| doc.syntax)
            .collect();
        assert_eq!(
            rows,
            [
                "NOP",
                "WAIT",
                "INC",
                "LD Mn",
                "ST Mn",
                "LDI A, ii",
                "CPI ii",
                "IN p",
                "JMP aa",
                "JZ aa",
                "JC aa"
            ]
        );
    }
}
