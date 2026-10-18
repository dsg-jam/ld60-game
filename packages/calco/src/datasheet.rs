//! Tables for the printed datasheet. They match `docs/isa/calco.md`.

use self::FlagEffect::{Cleared, FromResult, Kept, Set};
use crate::Cond;

pub const INSTRUCTIONS: &[InstructionDoc] = &[
    op("00", "NOP", "Nothing", Kept, Kept),
    op("01", "HLT", "Wait for a key press", Kept, Kept),
    op("02", "RET", "PC <- pop", Kept, Kept),
    op("03", "SWP", "Swap A and B", Kept, Kept),
    op("04", "CLC", "C <- 0", Kept, Cleared),
    op("05", "SEC", "C <- 1", Kept, Set),
    op("06", "INC", "A <- A + 1", FromResult, Kept),
    op("07", "DEC", "A <- A - 1", FromResult, Kept),
    op(
        "08",
        "DA",
        "Decimal adjust after ADC",
        FromResult,
        FromResult,
    ),
    op("09", "DS", "Decimal adjust after SBC", FromResult, Kept),
    op("1n", "LD Mn", "A <- Mn", Kept, Kept),
    op("2n", "ST Mn", "Mn <- A", Kept, Kept),
    op("30 ii", "LDI A, ii", "A <- ii", Kept, Kept),
    op("31 ii", "LDI B, ii", "B <- ii", Kept, Kept),
    op(
        "32 ii",
        "CPI ii",
        "A - ii, result dropped",
        FromResult,
        FromResult,
    ),
    op("4n", "ADC Mn", "A <- A + Mn + C", FromResult, FromResult),
    op("5n", "SBC Mn", "A <- A - Mn - C", FromResult, FromResult),
    op(
        "6n",
        "CMP Mn",
        "A - Mn, result dropped",
        FromResult,
        FromResult,
    ),
    op("7p", "IN p", "A <- port p", Kept, Kept),
    op("8p", "OUT p", "port p <- A", Kept, Kept),
    op("9n", "LD [B+n]", "A <- M(B+n)", Kept, Kept),
    op("An", "ST [B+n]", "M(B+n) <- A", Kept, Kept),
    op("Bc aa", "Jcc aa", "If c holds, PC <- aa", Kept, Kept),
    op(
        "Cn",
        "ADC [B+n]",
        "A <- A + M(B+n) + C",
        FromResult,
        FromResult,
    ),
    op(
        "Dn",
        "SBC [B+n]",
        "A <- A - M(B+n) - C",
        FromResult,
        FromResult,
    ),
    op(
        "En",
        "CMP [B+n]",
        "A - M(B+n), result dropped",
        FromResult,
        FromResult,
    ),
    op("F0 aa", "CALL aa", "push PC + 2, PC <- aa", Kept, Kept),
    op("F1 aa", "DJNZ aa", "B <- B - 1, jump if B != 0", Kept, Kept),
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
        function: "Chip revision. Rev A returns 0x10.",
    },
    PortDoc {
        port: 0x0,
        name: "ANN",
        output: true,
        function: "Annunciators. The board decides what each bit shows.",
    },
];

pub const NOTES: &[&str] = &[
    "There is no plain ADD or SUB. Use CLC first, then ADC or SBC.",
    "INC and DEC do not change C.",
    "DJNZ with B = 0 loops 256 times.",
    "The stack holds two return addresses. A third CALL overwrites the oldest.",
    "Indexed addresses wrap inside RAM. [B+3] with B = 0x0E reads M1.",
    "DA: if A >= 10, A <- A - 10 and C <- 1, else C <- 0.",
    "DS: if C = 1, A <- A + 10. C stays set.",
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
    pub hex: &'static str,
    pub syntax: &'static str,
    pub operation: &'static str,
    pub z: FlagEffect,
    pub c: FlagEffect,
}

#[derive(Clone, Copy, Debug)]
pub struct PortDoc {
    pub port: u8,
    pub name: &'static str,
    pub output: bool,
    pub function: &'static str,
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
}

const fn op(
    hex: &'static str,
    syntax: &'static str,
    operation: &'static str,
    z: FlagEffect,
    c: FlagEffect,
) -> InstructionDoc {
    InstructionDoc {
        hex,
        syntax,
        operation,
        z,
        c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
