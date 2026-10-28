//! Tables for the printed datasheet. They match `docs/isa/calco.md`.
//!
//! Each entry names an [`Op`], so a datasheet for one chip can show only
//! what [`Revision::supports`](crate::Revision::supports). The meanings are
//! for players: one sentence, starting with a verb, with a real example.

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
        operation: "Nothing",
        z: Kept,
        c: Kept,
        group: None,
        example: "NOP",
        meaning: "Do nothing.",
    },
    InstructionDoc {
        op: Op::Wait,
        hex: "01",
        syntax: "WAIT",
        operation: "Wait for a key press",
        z: Kept,
        c: Kept,
        group: Some(Keys),
        example: "WAIT",
        meaning: "Wait for the next key press.",
    },
    InstructionDoc {
        op: Op::Ret,
        hex: "02",
        syntax: "RET",
        operation: "PC <- pop",
        z: Kept,
        c: Kept,
        group: Some(Routines),
        example: "RET",
        meaning: "Go back to the line after the CALL.",
    },
    InstructionDoc {
        op: Op::Swp,
        hex: "03",
        syntax: "SWP",
        operation: "Swap A and B",
        z: Kept,
        c: Kept,
        group: Some(Numbers),
        example: "SWP",
        meaning: "Swap the numbers in A and B.",
    },
    InstructionDoc {
        op: Op::Clc,
        hex: "04",
        syntax: "CLC",
        operation: "C <- 0",
        z: Kept,
        c: Cleared,
        group: Some(Math),
        example: "CLC",
        meaning: "Clear the carry. Do this before ADC or SBC.",
    },
    InstructionDoc {
        op: Op::Sec,
        hex: "05",
        syntax: "SEC",
        operation: "C <- 1",
        z: Kept,
        c: Set,
        group: Some(Math),
        example: "SEC",
        meaning: "Set the carry.",
    },
    InstructionDoc {
        op: Op::Inc,
        hex: "06",
        syntax: "INC",
        operation: "A <- A + 1",
        z: FromResult,
        c: Kept,
        group: Some(Numbers),
        example: "INC",
        meaning: "Add 1 to A.",
    },
    InstructionDoc {
        op: Op::Dec,
        hex: "07",
        syntax: "DEC",
        operation: "A <- A - 1",
        z: FromResult,
        c: Kept,
        group: Some(Numbers),
        example: "DEC",
        meaning: "Take 1 from A.",
    },
    InstructionDoc {
        op: Op::Da,
        hex: "08",
        syntax: "DA",
        operation: "Decimal adjust after ADC",
        z: FromResult,
        c: FromResult,
        group: Some(Math),
        example: "DA",
        meaning: "After ADC: if A is 10 or more, take 10 off and carry 1.",
    },
    InstructionDoc {
        op: Op::Ds,
        hex: "09",
        syntax: "DS",
        operation: "Decimal adjust after SBC",
        z: FromResult,
        c: Kept,
        group: Some(Math),
        example: "DS",
        meaning: "After SBC: if 1 was borrowed, add 10 back to A.",
    },
    InstructionDoc {
        op: Op::Ld(DIRECT),
        hex: "1n",
        syntax: "LD Mn",
        operation: "A <- Mn",
        z: Kept,
        c: Kept,
        group: Some(Numbers),
        example: "LD M2",
        meaning: "Copy cell M2 to A.",
    },
    InstructionDoc {
        op: Op::St(DIRECT),
        hex: "2n",
        syntax: "ST Mn",
        operation: "Mn <- A",
        z: Kept,
        c: Kept,
        group: Some(Numbers),
        example: "ST M2",
        meaning: "Copy A to cell M2.",
    },
    InstructionDoc {
        op: Op::LdiA(0),
        hex: "30 ii",
        syntax: "LDI A, ii",
        operation: "A <- ii",
        z: Kept,
        c: Kept,
        group: Some(Numbers),
        example: "LDI A, 5",
        meaning: "Put 5 in A.",
    },
    InstructionDoc {
        op: Op::LdiB(0),
        hex: "31 ii",
        syntax: "LDI B, ii",
        operation: "B <- ii",
        z: Kept,
        c: Kept,
        group: Some(Loops),
        example: "LDI B, 4",
        meaning: "Put 4 in B.",
    },
    InstructionDoc {
        op: Op::Cpi(0),
        hex: "32 ii",
        syntax: "CPI ii",
        operation: "A - ii, result dropped",
        z: FromResult,
        c: FromResult,
        group: Some(Jumps),
        example: "CPI 10",
        meaning: "Compare A with 10. A does not change.",
    },
    InstructionDoc {
        op: Op::Adc(DIRECT),
        hex: "4n",
        syntax: "ADC Mn",
        operation: "A <- A + Mn + C",
        z: FromResult,
        c: FromResult,
        group: Some(Math),
        example: "ADC M2",
        meaning: "Add cell M2 and the carry to A.",
    },
    InstructionDoc {
        op: Op::Sbc(DIRECT),
        hex: "5n",
        syntax: "SBC Mn",
        operation: "A <- A - Mn - C",
        z: FromResult,
        c: FromResult,
        group: Some(Math),
        example: "SBC M2",
        meaning: "Take cell M2 and the carry from A.",
    },
    InstructionDoc {
        op: Op::Cmp(DIRECT),
        hex: "6n",
        syntax: "CMP Mn",
        operation: "A - Mn, result dropped",
        z: FromResult,
        c: FromResult,
        group: Some(Jumps),
        example: "CMP M2",
        meaning: "Compare A with cell M2. A does not change.",
    },
    InstructionDoc {
        op: Op::In(0),
        hex: "7p",
        syntax: "IN p",
        operation: "A <- port p",
        z: Kept,
        c: Kept,
        group: Some(Keys),
        example: "IN KEY",
        meaning: "Put the code of the pressed key in A. The gadget manual lists the codes.",
    },
    InstructionDoc {
        op: Op::Out(0),
        hex: "8p",
        syntax: "OUT p",
        operation: "port p <- A",
        z: Kept,
        c: Kept,
        group: Some(Lights),
        example: "OUT ANN",
        meaning: "Light the marks of the display. Each bit of A is one mark.",
    },
    InstructionDoc {
        op: Op::Ld(INDEXED),
        hex: "9n",
        syntax: "LD [B+n]",
        operation: "A <- M(B+n)",
        z: Kept,
        c: Kept,
        group: Some(Loops),
        example: "LD [B+2]",
        meaning: "Copy the cell 2 after B to A. With B at 3, that is M5.",
    },
    InstructionDoc {
        op: Op::St(INDEXED),
        hex: "An",
        syntax: "ST [B+n]",
        operation: "M(B+n) <- A",
        z: Kept,
        c: Kept,
        group: Some(Loops),
        example: "ST [B+2]",
        meaning: "Copy A to the cell 2 after B.",
    },
    InstructionDoc {
        op: Op::Jump(Cond::Always, 0),
        hex: "B0 aa",
        syntax: "JMP aa",
        operation: "PC <- aa",
        z: Kept,
        c: Kept,
        group: Some(Jumps),
        example: "JMP count",
        meaning: "Go to the line labeled count.",
    },
    InstructionDoc {
        op: Op::Jump(Cond::Zero, 0),
        hex: "B1 aa",
        syntax: "JZ aa",
        operation: "If Z = 1, PC <- aa",
        z: Kept,
        c: Kept,
        group: Some(Jumps),
        example: "JZ done",
        meaning: "After a compare: go to done if A was equal.",
    },
    InstructionDoc {
        op: Op::Jump(Cond::NotZero, 0),
        hex: "B2 aa",
        syntax: "JNZ aa",
        operation: "If Z = 0, PC <- aa",
        z: Kept,
        c: Kept,
        group: Some(Jumps),
        example: "JNZ done",
        meaning: "After a compare: go to done if A was not equal.",
    },
    InstructionDoc {
        op: Op::Jump(Cond::Carry, 0),
        hex: "B3 aa",
        syntax: "JC aa",
        operation: "If C = 1, PC <- aa",
        z: Kept,
        c: Kept,
        group: Some(Jumps),
        example: "JC done",
        meaning: "After a compare: go to done if A was lower.",
    },
    InstructionDoc {
        op: Op::Jump(Cond::NoCarry, 0),
        hex: "B4 aa",
        syntax: "JNC aa",
        operation: "If C = 0, PC <- aa",
        z: Kept,
        c: Kept,
        group: Some(Jumps),
        example: "JNC done",
        meaning: "After a compare: go to done if A was not lower.",
    },
    InstructionDoc {
        op: Op::Adc(INDEXED),
        hex: "Cn",
        syntax: "ADC [B+n]",
        operation: "A <- A + M(B+n) + C",
        z: FromResult,
        c: FromResult,
        group: Some(Loops),
        example: "ADC [B+2]",
        meaning: "Add the cell 2 after B and the carry to A.",
    },
    InstructionDoc {
        op: Op::Sbc(INDEXED),
        hex: "Dn",
        syntax: "SBC [B+n]",
        operation: "A <- A - M(B+n) - C",
        z: FromResult,
        c: FromResult,
        group: Some(Loops),
        example: "SBC [B+2]",
        meaning: "Take the cell 2 after B and the carry from A.",
    },
    InstructionDoc {
        op: Op::Cmp(INDEXED),
        hex: "En",
        syntax: "CMP [B+n]",
        operation: "A - M(B+n), result dropped",
        z: FromResult,
        c: FromResult,
        group: Some(Loops),
        example: "CMP [B+2]",
        meaning: "Compare A with the cell 2 after B.",
    },
    InstructionDoc {
        op: Op::Call(0),
        hex: "F0 aa",
        syntax: "CALL aa",
        operation: "push PC + 2, PC <- aa",
        z: Kept,
        c: Kept,
        group: Some(Routines),
        example: "CALL show",
        meaning: "Run the lines from show until RET, then come back.",
    },
    InstructionDoc {
        op: Op::Djnz(0),
        hex: "F1 aa",
        syntax: "DJNZ aa",
        operation: "B <- B - 1, jump if B != 0",
        z: Kept,
        c: Kept,
        group: Some(Loops),
        example: "DJNZ next",
        meaning: "Take 1 from B, then go to next unless B is 0.",
    },
];

pub const PORTS: &[PortDoc] = &[
    PortDoc {
        port: 0x0,
        name: "KEY",
        output: false,
        function: "Code of the last key, 255 if none. Reading it takes it.",
    },
    PortDoc {
        port: 0xF,
        name: "REV",
        output: false,
        function: "Chip revision: 16 on Rev A, 32 on Rev B, 48 on Rev C.",
    },
    PortDoc {
        port: 0x0,
        name: "ANN",
        output: true,
        function: "Marks of the display. The board decides what each bit shows.",
    },
];

/// Notes go right under the instruction they are about.
pub const NOTES: &[NoteDoc] = &[
    note(
        Op::Adc(DIRECT),
        "There is no plain add. Use CLC first, then ADC.",
    ),
    note(
        Op::Sbc(DIRECT),
        "There is no plain subtract. Use CLC first, then SBC.",
    ),
    note(Op::Djnz(0), "With B at 0, DJNZ goes round 256 times."),
    note(
        Op::Call(0),
        "Only two CALLs can wait to come back. A third one forgets the oldest.",
    ),
    note(
        Op::Ld(INDEXED),
        "Cells wrap around: with B at 14, [B+3] is M1.",
    ),
    note(Op::Ds, "DS keeps the carry set."),
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
    pub operation: &'static str,
    pub z: FlagEffect,
    pub c: FlagEffect,
    /// Where the instruction goes in the plain list. `None` keeps it to
    /// the table.
    pub group: Option<Group>,
    /// The syntax with real operands, like `LDI A, 5`.
    pub example: &'static str,
    /// What the example does, for players.
    pub meaning: &'static str,
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

const fn note(op: Op, text: &'static str) -> NoteDoc {
    NoteDoc { op, text }
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
            if cond != Cond::Always {
                let operation = format!("If {}, PC <- aa", cond.description());
                assert_eq!(rows[0].operation, operation);
            }
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
