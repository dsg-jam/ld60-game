use std::collections::{HashMap, HashSet};

use crate::variant::line::Line;

/// Calls nested deeper than this count as unknown code.
const MAX_DEPTH: usize = 4;

/// The Z and C flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Flags {
    pub(crate) zero: bool,
    pub(crate) carry: bool,
}

impl Flags {
    const BOTH: Self = Self {
        zero: true,
        carry: true,
    };

    /// Flags of the compare at line `at` that a later instruction may read.
    ///
    /// Follows fall through, jumps and calls until both flags are written
    /// again. A flag that is still live at a `RET` to an unknown caller or at
    /// an unknown target counts as read.
    pub(crate) fn read_after(lines: &[Line<'_>], at: usize) -> Self {
        let labels = lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| Some((line.label()?, i)))
            .collect();
        let walk = Walk::run(lines, &labels, at + 1, Self::BOTH, 0);
        walk.read.or(walk.returned)
    }

    fn of(mnemonic: &str) -> (Self, Self) {
        let flags = |zero, carry| Self { zero, carry };
        let none = flags(false, false);
        match mnemonic {
            "JZ" | "JNZ" => (flags(true, false), none),
            "JC" | "JNC" => (flags(false, true), none),
            "ADC" | "SBC" => (flags(false, true), Self::BOTH),
            "DS" => (flags(false, true), flags(true, false)),
            "DA" | "CPI" | "CMP" => (none, Self::BOTH),
            "INC" | "DEC" => (none, flags(true, false)),
            "CLC" | "SEC" => (none, flags(false, true)),
            _ => (none, none),
        }
    }

    fn any(self) -> bool {
        self.zero || self.carry
    }

    fn and(self, other: Self) -> Self {
        Self {
            zero: self.zero && other.zero,
            carry: self.carry && other.carry,
        }
    }

    fn or(self, other: Self) -> Self {
        Self {
            zero: self.zero || other.zero,
            carry: self.carry || other.carry,
        }
    }

    fn without(self, other: Self) -> Self {
        Self {
            zero: self.zero && !other.zero,
            carry: self.carry && !other.carry,
        }
    }
}

/// What happens to live flags from one start line on.
#[derive(Clone, Copy, Debug, Default)]
struct Walk {
    read: Flags,
    /// Flags still live at a `RET`.
    returned: Flags,
}

impl Walk {
    fn run(
        lines: &[Line<'_>],
        labels: &HashMap<&str, usize>,
        start: usize,
        live: Flags,
        depth: usize,
    ) -> Self {
        let mut walk = Self::default();
        if depth > MAX_DEPTH {
            walk.read = live;
            return walk;
        }
        let mut seen = HashSet::new();
        let mut todo = vec![(start, live)];
        while let Some((i, live)) = todo.pop() {
            if !live.any() || !seen.insert((i, live)) {
                continue;
            }
            let Some(line) = lines.get(i) else {
                continue;
            };
            let Some(mnemonic) = line.mnemonic() else {
                todo.push((i + 1, live));
                continue;
            };
            let (reads, writes) = Flags::of(&mnemonic);
            walk.read = walk.read.or(live.and(reads));
            let live = live.without(writes);
            let target = line
                .operands()
                .first()
                .and_then(|name| labels.get(name).copied());
            match (mnemonic.as_str(), target) {
                ("RET", _) => walk.returned = walk.returned.or(live),
                ("JMP", Some(target)) => todo.push((target, live)),
                ("CALL", Some(target)) => {
                    let call = Self::run(lines, labels, target, live, depth + 1);
                    walk.read = walk.read.or(call.read);
                    todo.push((i + 1, call.returned));
                }
                ("JZ" | "JNZ" | "JC" | "JNC" | "DJNZ", Some(target)) => {
                    todo.push((target, live));
                    todo.push((i + 1, live));
                }
                ("JMP" | "CALL" | "JZ" | "JNZ" | "JC" | "JNC" | "DJNZ", None) => {
                    walk.read = walk.read.or(live);
                }
                _ => todo.push((i + 1, live)),
            }
        }
        walk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_after(source: &str, at: usize) -> Flags {
        let lines: Vec<Line<'_>> = source.lines().map(Line::new).collect();
        Flags::read_after(&lines, at)
    }

    #[test]
    fn finds_every_reader_until_both_flags_are_written() {
        let source = "\
        CPI  0x0C
        JZ   eq
        JC   low
        CPI  0
eq:     CPI  0
low:    CPI  0
";
        assert_eq!(read_after(source, 0), Flags::BOTH);
    }

    #[test]
    fn follows_calls_that_write_the_flags() {
        let source = "\
        CPI  0x0A
        JZ   plus
        WAIT
plus:   CALL clear
        RET
clear:  CLC
        RET
";
        let flags = read_after(source, 0);
        assert_eq!((flags.zero, flags.carry), (true, false));
    }

    #[test]
    fn unknown_targets_and_returns_read_everything() {
        assert_eq!(read_after("CPI 1\nJZ away\n", 0), Flags::BOTH);
        assert_eq!(read_after("CPI 1\nRET\n", 0), Flags::BOTH);
    }
}
