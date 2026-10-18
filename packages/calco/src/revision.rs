use std::fmt;

use crate::encoder::MNEMONICS;
use crate::{Addr, Calco, Cond, Op};

const REV_A_MNEMONICS: &[&str] = &[
    "NOP", "HLT", "INC", "LD", "ST", "LDI", "CPI", "IN", "JMP", "JZ", "JC",
];

/// Chip revision. Later revisions support more instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Revision {
    /// Only what a simple counter needs. No B register, stack or arithmetic.
    A,
    /// The full instruction set.
    B,
    /// Same instructions as Rev B.
    C,
}

impl Revision {
    pub const ALL: [Revision; 3] = [Self::A, Self::B, Self::C];

    /// Value returned by `IN REV`.
    pub fn id(self) -> u8 {
        match self {
            Self::A => 0x10,
            Self::B => 0x20,
            Self::C => 0x30,
        }
    }

    /// Decodes like this chip does. Unsupported ops come back as [`Op::Reserved`].
    pub fn decode(self, first: u8, second: u8) -> Op {
        match Op::decode(first, second) {
            op if self.supports(op) => op,
            _ => Op::Reserved(first),
        }
    }

    /// The instruction set as this revision implements it.
    pub fn isa(self) -> &'static Calco {
        match self {
            Self::A => &Calco(Self::A),
            Self::B => &Calco(Self::B),
            Self::C => &Calco(Self::C),
        }
    }

    /// Mnemonics with at least one supported form.
    pub fn mnemonics(self) -> &'static [&'static str] {
        match self {
            Self::A => REV_A_MNEMONICS,
            Self::B | Self::C => MNEMONICS,
        }
    }

    /// Whether the chip runs `op`. Unsupported opcodes act like reserved ones.
    pub fn supports(self, op: Op) -> bool {
        match self {
            Self::A => matches!(
                op,
                Op::Nop
                    | Op::Hlt
                    | Op::Inc
                    | Op::Ld(Addr::Direct(_))
                    | Op::St(Addr::Direct(_))
                    | Op::LdiA(_)
                    | Op::Cpi(_)
                    | Op::In(_)
                    | Op::Jump(Cond::Always | Cond::Zero | Cond::Carry, _)
            ),
            Self::B | Self::C => !matches!(op, Op::Reserved(_)),
        }
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
