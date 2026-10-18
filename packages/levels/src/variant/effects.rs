use super::line::{Line, cell};

const A: u32 = 1;
const B: u32 = 1 << 1;
const Z: u32 = 1 << 2;
const C: u32 = 1 << 3;
const MEMORY: u32 = 0xFFFF << 4;

/// Registers, flags and RAM cells a CalCo instruction reads and writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Effects {
    reads: u32,
    writes: u32,
}

impl Effects {
    /// Returns `None` for instructions that must stay in place, like jumps and I/O.
    pub(crate) fn of(line: &Line<'_>) -> Option<Self> {
        let mnemonic = line.mnemonic()?;
        let operands = line.operands();
        let (reads, writes) = match (mnemonic.as_str(), operands.as_slice()) {
            ("NOP", []) => (0, 0),
            ("SWP", []) => (A | B, A | B),
            ("CLC" | "SEC", []) => (0, C),
            ("INC" | "DEC", []) => (A, A | Z),
            ("DA", []) => (A, A | Z | C),
            ("DS", []) => (A | C, A | Z),
            ("LDI", ["A" | "a", _]) => (0, A),
            ("LDI", ["B" | "b", _]) => (0, B),
            ("CPI", [_]) => (A, Z | C),
            ("LD", [m]) => {
                let (pointer, cells) = memory(m)?;
                (pointer | cells, A)
            }
            ("ST", [m]) => {
                let (pointer, cells) = memory(m)?;
                (A | pointer, cells)
            }
            ("ADC" | "SBC", [m]) => {
                let (pointer, cells) = memory(m)?;
                (A | C | pointer | cells, A | Z | C)
            }
            ("CMP", [m]) => {
                let (pointer, cells) = memory(m)?;
                (A | pointer | cells, Z | C)
            }
            _ => return None,
        };
        Some(Self { reads, writes })
    }

    pub(crate) fn commutes(self, other: Self) -> bool {
        self.writes & (other.reads | other.writes) == 0 && other.writes & self.reads == 0
    }
}

/// Returns the pointer register it reads and the cells it may touch.
fn memory(operand: &str) -> Option<(u32, u32)> {
    if operand.starts_with('[') {
        return Some((B, MEMORY));
    }
    cell(operand).map(|n| (0, 1 << (4 + n)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effects(text: &str) -> Option<Effects> {
        Effects::of(&Line::new(text))
    }

    #[test]
    fn independent_instructions_commute() {
        let commutes = |a, b| effects(a).unwrap().commutes(effects(b).unwrap());
        assert!(commutes("LDI B, 6", "CLC"));
        assert!(commutes("ST M1", "ST M2"));
        assert!(commutes("LDI A, 0", "LDI B, 6"));
        assert!(!commutes("LD M2", "ST M1"));
        assert!(!commutes("ST M1", "LD M1"));
        assert!(!commutes("LDI B, 6", "ST [B]"));
        assert!(!commutes("CLC", "ADC M1"));
    }

    #[test]
    fn control_flow_and_io_stay_in_place() {
        for text in [
            "JMP main",
            "RET",
            "HLT",
            "IN KEY",
            "DJNZ loop",
            "; only a comment",
        ] {
            assert_eq!(effects(text), None, "{text}");
        }
    }
}
