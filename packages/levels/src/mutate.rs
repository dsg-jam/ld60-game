use std::ops::Range;

use crate::variant::effects::Effects;
use crate::variant::line::{Line, cell, fit, immediate, number};

/// A kind of bug that changes one line of working firmware.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operator {
    /// An immediate one too high or too low, like `CPI 9` for `CPI 10`.
    OffByOne,
    /// A jump that tests the opposite condition, like `JNZ` for `JZ`.
    FlippedJump,
    /// A neighbouring RAM cell, like `LD M3` for `LD M2`.
    WrongCell,
    /// An instruction that is missing.
    DroppedInstruction,
    /// An instruction moved below the next one, which depends on it.
    SwappedInstructions,
    /// A jump to another label, like `JZ store` for `JZ main`.
    WrongTarget,
    /// A key check with the code of another key of the board, like
    /// `CPI 0x0B` for `CPI 0x0C`.
    WrongKey,
    /// Working firmware on a board with a stuck RAM bit in a cell it uses.
    /// The fix moves that cell to a free one. It is not in
    /// [`Operator::ALL`], since it changes no line of its own.
    MovedCell,
}

/// One operator applied at one place in the firmware.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mutation {
    pub(crate) operator: Operator,
    /// The first line it replaces.
    pub(crate) line: usize,
    replaced: usize,
    new: Vec<String>,
}

impl Operator {
    pub const ALL: [Self; 7] = [
        Self::OffByOne,
        Self::FlippedJump,
        Self::WrongCell,
        Self::DroppedInstruction,
        Self::SwappedInstructions,
        Self::WrongTarget,
        Self::WrongKey,
    ];

    /// The mutations this operator can make at line `at`. `keys` are the
    /// codes of the keys that are not digits.
    fn at(self, lines: &[String], at: usize, keys: &[u8]) -> Vec<Mutation> {
        let text = &lines[at];
        let line = Line::new(text);
        let Some(mnemonic) = line.mnemonic() else {
            return Vec::new();
        };
        let operands = line.operands();
        let replace = |new: Vec<String>| Mutation {
            operator: self,
            line: at,
            replaced: 1,
            new,
        };
        let replace_operand = |operand: &str, values: Vec<String>| {
            values
                .into_iter()
                .filter_map(|value| fit(text, operand, &value))
                .map(|new| replace(vec![new]))
                .collect()
        };
        match (self, mnemonic.as_str(), operands.as_slice()) {
            (Self::OffByOne, "LDI", [_, value]) | (Self::OffByOne, "CPI", [value]) => {
                let Some(old) = number(value) else {
                    return Vec::new();
                };
                let values = [old.checked_add(1), old.checked_sub(1)];
                let values = values.into_iter().flatten();
                replace_operand(value, values.map(|n| immediate(value, n)).collect())
            }
            (Self::FlippedJump, "JZ" | "JNZ" | "JC" | "JNC", _) => {
                let flipped = match mnemonic.as_str() {
                    "JZ" => "JNZ",
                    "JNZ" => "JZ",
                    "JC" => "JNC",
                    _ => "JC",
                };
                let old = &line.rest[..mnemonic.len()];
                fit(text, old, flipped)
                    .map(|new| replace(vec![new]))
                    .into_iter()
                    .collect()
            }
            (Self::WrongCell, "LD" | "ST" | "ADC" | "SBC" | "CMP", [operand]) => {
                replace_operand(operand, neighbour_cells(operand))
            }
            (Self::DroppedInstruction, mnemonic, _) if mnemonic != "HLT" => {
                let label = line.label().map(|_| line.prefix.trim_end().to_owned());
                vec![replace(label.into_iter().collect())]
            }
            (Self::SwappedInstructions, ..) => Mutation::swap(lines, at).into_iter().collect(),
            (Self::WrongTarget, "JMP" | "JZ" | "JNZ" | "JC" | "JNC", [target]) => {
                let labels = lines
                    .iter()
                    .filter_map(|line| Line::new(line).label())
                    .filter(|label| label != target && !returns(lines, label))
                    .map(String::from)
                    .collect();
                replace_operand(target, labels)
            }
            (Self::WrongKey, "CPI", [value]) if checks_equal(lines, at) => {
                let Some(old) = number(value).filter(|code| keys.contains(code)) else {
                    return Vec::new();
                };
                let others = keys.iter().filter(|&&code| code != old);
                replace_operand(value, others.map(|&n| immediate(value, n)).collect())
            }
            _ => Vec::new(),
        }
    }
}

impl Mutation {
    /// Every mutation `operator` can make in `lines`. `keys` are the codes
    /// of the keys that are not digits.
    pub(crate) fn candidates(lines: &[String], operator: Operator, keys: &[u8]) -> Vec<Self> {
        (0..lines.len())
            .flat_map(|at| operator.at(lines, at, keys))
            .collect()
    }

    /// Moves the instruction at `at` below the next one, if the next one depends on it.
    ///
    /// Neither may have a label, so the fix moves one whole line.
    fn swap(lines: &[String], at: usize) -> Option<Self> {
        let (first, second) = (Line::new(&lines[at]), Line::new(lines.get(at + 1)?));
        let (a, b) = (Effects::of(&first)?, Effects::of(&second)?);
        if first.label().is_some() || second.label().is_some() || a.commutes(b) {
            return None;
        }
        Some(Self {
            operator: Operator::SwappedInstructions,
            line: at,
            replaced: 2,
            new: vec![
                format!("{}{}", first.prefix, second.rest),
                format!("{}{}", second.prefix, first.rest),
            ],
        })
    }

    /// The lines it replaces.
    pub(crate) fn span(&self) -> Range<usize> {
        self.line..self.line + self.replaced
    }

    /// Whether at least one untouched line lies between the two.
    pub(crate) fn apart(&self, other: &Self) -> bool {
        self.span().end < other.line || other.span().end < self.line
    }

    /// `lines` with all `mutations` applied. They must be [`Mutation::apart`].
    pub(crate) fn apply(lines: &[String], mutations: &[&Self]) -> String {
        let mut lines = lines.to_vec();
        let mut mutations = mutations.to_vec();
        mutations.sort_by_key(|m| std::cmp::Reverse(m.line));
        for mutation in mutations {
            lines.splice(mutation.span(), mutation.new.iter().cloned());
        }
        lines.iter().map(|line| format!("{line}\n")).collect()
    }
}

/// Whether the code at `label` can reach a `RET` without a `CALL` first. A
/// jump there would return to whatever the stack held.
fn returns(lines: &[String], label: &str) -> bool {
    let find = |name: &str| {
        lines
            .iter()
            .position(|line| Line::new(line).label() == Some(name))
    };
    let mut todo: Vec<usize> = find(label).into_iter().collect();
    let mut seen = vec![false; lines.len()];
    while let Some(at) = todo.pop() {
        if at >= lines.len() || seen[at] {
            continue;
        }
        seen[at] = true;
        let line = Line::new(&lines[at]);
        let mnemonic = line.mnemonic().unwrap_or_default();
        match mnemonic.as_str() {
            "RET" => return true,
            "JMP" | "JZ" | "JNZ" | "JC" | "JNC" | "DJNZ" => {
                todo.extend(line.operands().last().and_then(|target| find(target)));
            }
            _ => {}
        }
        if mnemonic != "JMP" {
            todo.push(at + 1);
        }
    }
    false
}

/// Whether the line after `at` jumps on an equal or unequal compare.
fn checks_equal(lines: &[String], at: usize) -> bool {
    lines
        .get(at + 1)
        .and_then(|line| Line::new(line).mnemonic())
        .is_some_and(|mnemonic| mnemonic == "JZ" || mnemonic == "JNZ")
}

/// The cells next to a direct `Mn` or indexed `[B+n]` operand.
fn neighbour_cells(operand: &str) -> Vec<String> {
    if let Some(n) = cell(operand) {
        let near = [n.checked_add(1).filter(|&n| n < 16), n.checked_sub(1)];
        return near
            .into_iter()
            .flatten()
            .map(|n| format!("M{n:X}"))
            .collect();
    }
    let Some(inner) = operand.strip_prefix('[').and_then(|o| o.strip_suffix(']')) else {
        return Vec::new();
    };
    let (register, offset) = match inner.split_once('+') {
        Some((register, offset)) => (register.trim(), number(offset.trim())),
        None => (inner.trim(), Some(0)),
    };
    let Some(offset) = offset else {
        return Vec::new();
    };
    let near = [
        offset.checked_add(1).filter(|&n| n < 16),
        offset.checked_sub(1),
    ];
    near.into_iter()
        .flatten()
        .map(|n| match n {
            0 => format!("[{register}]"),
            n => format!("[{register}+{n}]"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mutate(source: &str, operator: Operator) -> Vec<String> {
        let lines: Vec<String> = source.lines().map(String::from).collect();
        Mutation::candidates(&lines, operator, &[0x0A, 0x0C])
            .iter()
            .map(|m| Mutation::apply(&lines, &[m]))
            .collect()
    }

    #[test]
    fn moves_immediates_by_one_in_their_style() {
        assert_eq!(
            mutate("  CPI 0x0A\n  LDI A, 0\n", Operator::OffByOne),
            [
                "  CPI 0x0B\n  LDI A, 0\n",
                "  CPI 0x09\n  LDI A, 0\n",
                "  CPI 0x0A\n  LDI A, 1\n"
            ]
        );
    }

    #[test]
    fn flips_conditional_jumps_only() {
        assert_eq!(
            mutate(
                "x: JZ   a ; z\n   JMP  a\n   JNC  a\n   JC a\n",
                Operator::FlippedJump
            ),
            [
                "x: JNZ  a ; z\n   JMP  a\n   JNC  a\n   JC a\n",
                "x: JZ   a ; z\n   JMP  a\n   JC   a\n   JC a\n"
            ]
        );
    }

    #[test]
    fn keeps_the_comment_column() {
        assert_eq!(
            mutate("LD [B]   ; a\nLD [B] ; b\n", Operator::WrongCell),
            ["LD [B+1] ; a\nLD [B] ; b\n"]
        );
        assert_eq!(
            mutate("CPI 10 ; a\nCPI 9 ; b\n", Operator::OffByOne),
            [
                "CPI 11 ; a\nCPI 9 ; b\n",
                "CPI 9  ; a\nCPI 9 ; b\n",
                "CPI 10 ; a\nCPI 8 ; b\n"
            ]
        );
    }

    #[test]
    fn picks_neighbouring_cells() {
        assert_eq!(
            mutate("ST M0\nLD [B+8]\nLD [B+1]\n", Operator::WrongCell),
            [
                "ST M1\nLD [B+8]\nLD [B+1]\n",
                "ST M0\nLD [B+9]\nLD [B+1]\n",
                "ST M0\nLD [B+7]\nLD [B+1]\n",
                "ST M0\nLD [B+8]\nLD [B+2]\n",
                "ST M0\nLD [B+8]\nLD [B]\n",
            ]
        );
    }

    #[test]
    fn keeps_the_label_of_a_dropped_instruction() {
        assert_eq!(
            mutate(
                "main: HLT\nloop: INC\n  ST M1\n",
                Operator::DroppedInstruction
            ),
            ["main: HLT\nloop:\n  ST M1\n", "main: HLT\nloop: INC\n"]
        );
    }

    #[test]
    fn jumps_to_other_labels() {
        assert_eq!(
            mutate("a:  JZ  b\nbb: JMP a\nb:  HLT\n", Operator::WrongTarget),
            [
                "a:  JZ  a\nbb: JMP a\nb:  HLT\n",
                "a:  JZ  bb\nbb: JMP a\nb:  HLT\n",
                "a:  JZ  b\nbb: JMP bb\nb:  HLT\n",
                "a:  JZ  b\nbb: JMP b\nb:  HLT\n",
            ]
        );
    }

    #[test]
    fn never_jumps_into_a_routine() {
        assert_eq!(
            mutate(
                "main: JZ  a\n      CALL r\na:    JMP main\nr:    INC\n      RET\n",
                Operator::WrongTarget
            ),
            [
                "main: JZ  main\n      CALL r\na:    JMP main\nr:    INC\n      RET\n",
                "main: JZ  a\n      CALL r\na:    JMP a\nr:    INC\n      RET\n",
            ]
        );
    }

    #[test]
    fn checks_for_other_keys_only_before_equal_jumps() {
        assert_eq!(
            mutate(
                "  CPI 0x0C\n  JZ  a\n  CPI 0x0A\n  JC  b\n  CPI 3\n  JZ  c\n",
                Operator::WrongKey
            ),
            ["  CPI 0x0A\n  JZ  a\n  CPI 0x0A\n  JC  b\n  CPI 3\n  JZ  c\n"]
        );
    }

    #[test]
    fn swaps_only_dependent_neighbours() {
        assert_eq!(
            mutate(
                "a: LD M2\n   INC\n   ST M1\n   ST M2\n",
                Operator::SwappedInstructions
            ),
            ["a: LD M2\n   ST M1\n   INC\n   ST M2\n"]
        );
    }
}
