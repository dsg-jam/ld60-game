//! Tables for the printed datasheet. They match `docs/isa/calco.md`.
//!
//! Each entry names an [`Op`], so a datasheet for one chip can show only
//! what [`Revision::supports`](crate::Revision::supports).

use self::FlagEffect::{Cleared, FromResult, Kept, Set};
use crate::{Addr, Cond, Op};

const DIRECT: Addr = Addr::Direct(0);
const INDEXED: Addr = Addr::Indexed(0);

pub const INSTRUCTIONS: &[InstructionDoc] = &[
    op(Op::Nop, "00", "NOP", "Nothing", Kept, Kept),
    op(Op::Hlt, "01", "HLT", "Wait for a key press", Kept, Kept),
    op(Op::Ret, "02", "RET", "PC <- pop", Kept, Kept),
    op(Op::Swp, "03", "SWP", "Swap A and B", Kept, Kept),
    op(Op::Clc, "04", "CLC", "C <- 0", Kept, Cleared),
    op(Op::Sec, "05", "SEC", "C <- 1", Kept, Set),
    op(Op::Inc, "06", "INC", "A <- A + 1", FromResult, Kept),
    op(Op::Dec, "07", "DEC", "A <- A - 1", FromResult, Kept),
    op(
        Op::Da,
        "08",
        "DA",
        "Decimal adjust after ADC",
        FromResult,
        FromResult,
    ),
    op(
        Op::Ds,
        "09",
        "DS",
        "Decimal adjust after SBC",
        FromResult,
        Kept,
    ),
    op(Op::Ld(DIRECT), "1n", "LD Mn", "A <- Mn", Kept, Kept),
    op(Op::St(DIRECT), "2n", "ST Mn", "Mn <- A", Kept, Kept),
    op(Op::LdiA(0), "30 ii", "LDI A, ii", "A <- ii", Kept, Kept),
    op(Op::LdiB(0), "31 ii", "LDI B, ii", "B <- ii", Kept, Kept),
    op(
        Op::Cpi(0),
        "32 ii",
        "CPI ii",
        "A - ii, result dropped",
        FromResult,
        FromResult,
    ),
    op(
        Op::Adc(DIRECT),
        "4n",
        "ADC Mn",
        "A <- A + Mn + C",
        FromResult,
        FromResult,
    ),
    op(
        Op::Sbc(DIRECT),
        "5n",
        "SBC Mn",
        "A <- A - Mn - C",
        FromResult,
        FromResult,
    ),
    op(
        Op::Cmp(DIRECT),
        "6n",
        "CMP Mn",
        "A - Mn, result dropped",
        FromResult,
        FromResult,
    ),
    op(Op::In(0), "7p", "IN p", "A <- port p", Kept, Kept),
    op(Op::Out(0), "8p", "OUT p", "port p <- A", Kept, Kept),
    op(Op::Ld(INDEXED), "9n", "LD [B+n]", "A <- M(B+n)", Kept, Kept),
    op(Op::St(INDEXED), "An", "ST [B+n]", "M(B+n) <- A", Kept, Kept),
    op(
        Op::Jump(Cond::Always, 0),
        "Bc aa",
        "Jcc aa",
        "If c holds, PC <- aa",
        Kept,
        Kept,
    ),
    op(
        Op::Adc(INDEXED),
        "Cn",
        "ADC [B+n]",
        "A <- A + M(B+n) + C",
        FromResult,
        FromResult,
    ),
    op(
        Op::Sbc(INDEXED),
        "Dn",
        "SBC [B+n]",
        "A <- A - M(B+n) - C",
        FromResult,
        FromResult,
    ),
    op(
        Op::Cmp(INDEXED),
        "En",
        "CMP [B+n]",
        "A - M(B+n), result dropped",
        FromResult,
        FromResult,
    ),
    op(
        Op::Call(0),
        "F0 aa",
        "CALL aa",
        "push PC + 2, PC <- aa",
        Kept,
        Kept,
    ),
    op(
        Op::Djnz(0),
        "F1 aa",
        "DJNZ aa",
        "B <- B - 1, jump if B != 0",
        Kept,
        Kept,
    ),
];

pub const PORTS: &[PortDoc] = &[
    PortDoc {
        port: 0x0,
        name: "KEY",
        output: false,
        function: "Last key code from the board, 0xFF if none. Reading clears it.",
    },
    PortDoc {
        port: 0xF,
        name: "REV",
        output: false,
        function: "Chip revision. Rev A returns 0x10, Rev B 0x20, Rev C 0x30.",
    },
    PortDoc {
        port: 0x0,
        name: "ANN",
        output: true,
        function: "Annunciators. The board decides what each bit shows.",
    },
];

pub const NOTES: &[NoteDoc] = &[
    note(
        Op::Adc(DIRECT),
        "There is no plain ADD or SUB. Use CLC first, then ADC or SBC.",
    ),
    note(Op::Inc, "INC does not change C."),
    note(Op::Dec, "DEC does not change C."),
    note(Op::Djnz(0), "DJNZ with B = 0 loops 256 times."),
    note(
        Op::Call(0),
        "The stack holds two return addresses. A third CALL overwrites the oldest.",
    ),
    note(
        Op::Ld(INDEXED),
        "Indexed addresses wrap inside RAM. [B+3] with B = 0x0E reads M1.",
    ),
    note(
        Op::Da,
        "DA: if A >= 10, A <- A - 10 and C <- 1, else C <- 0.",
    ),
    note(Op::Ds, "DS: if C = 1, A <- A + 10. C stays set."),
];

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
    pub operation: &'static str,
    pub z: FlagEffect,
    pub c: FlagEffect,
}

/// A note that only applies if the chip supports `op`.
#[derive(Clone, Copy, Debug)]
pub struct NoteDoc {
    pub op: Op,
    pub text: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct PortDoc {
    pub port: u8,
    pub name: &'static str,
    pub output: bool,
    pub function: &'static str,
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
    /// When the jump is taken.
    pub fn description(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Zero => "Z = 1",
            Self::NotZero => "Z = 0",
            Self::Carry => "C = 1",
            Self::NoCarry => "C = 0",
        }
    }

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

const fn op(
    op: Op,
    hex: &'static str,
    syntax: &'static str,
    operation: &'static str,
    z: FlagEffect,
    c: FlagEffect,
) -> InstructionDoc {
    InstructionDoc {
        op,
        hex,
        syntax,
        operation,
        z,
        c,
    }
}

const fn note(op: Op, text: &'static str) -> NoteDoc {
    NoteDoc { op, text }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Revision;
    use crate::encoder::MNEMONICS;

    #[test]
    fn every_mnemonic_is_documented() {
        let documented = |mnemonic: &str| {
            INSTRUCTIONS
                .iter()
                .any(|doc| doc.syntax.split_whitespace().next() == Some(mnemonic))
                || Cond::ALL.iter().any(|cond| cond.mnemonic() == mnemonic)
        };
        let missing: Vec<_> = MNEMONICS.iter().filter(|m| !documented(m)).collect();
        assert!(
            missing.is_empty(),
            "missing from the datasheet: {missing:?}"
        );
    }

    #[test]
    fn rows_match_their_op() {
        for doc in INSTRUCTIONS {
            let hex = u8::from_str_radix(&doc.hex[..2].replace(['n', 'p', 'c'], "0"), 16);
            assert_eq!(hex.map(|first| Op::decode(first, 0)), Ok(doc.op), "{doc:?}");
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
                "HLT",
                "INC",
                "LD Mn",
                "ST Mn",
                "LDI A, ii",
                "CPI ii",
                "IN p",
                "Jcc aa"
            ]
        );
    }
}
