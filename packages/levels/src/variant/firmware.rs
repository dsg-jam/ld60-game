mod rewire;

use std::fmt;

use super::effects::Effects;
use super::line::Line;
use super::ram_map::RamMap;
use super::rng::Rng;

/// Firmware source as lines, so rewrites keep labels and comments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Firmware(Vec<String>);

impl Firmware {
    pub(crate) fn new(source: &str) -> Self {
        Self(source.lines().map(String::from).collect())
    }

    fn lines(&self) -> impl Iterator<Item = Line<'_>> {
        self.0.iter().map(|text| Line::new(text))
    }

    /// Cells that `[B+n]` can reach.
    ///
    /// B is assumed to run from 1 to the largest `LDI B` value.
    // ponytail: no data flow analysis. The test run in `Variant::new` catches wrong guesses.
    pub(crate) fn indexed_cells(&self) -> Vec<usize> {
        let mut offsets = Vec::new();
        let mut max_b = 0;
        for line in self.lines() {
            let mnemonic = line.mnemonic().unwrap_or_default();
            let operands = line.operands();
            if mnemonic == "SWP" {
                max_b = 0xFF;
            }
            if let ("LDI", ["B" | "b", value]) = (mnemonic.as_str(), operands.as_slice()) {
                max_b = max_b.max(ld60_asm::parse_number(value).unwrap_or(0xFF));
            }
            for operand in operands {
                if let Some(inner) = operand.strip_prefix('[') {
                    let offset = inner.trim_end_matches(']').split_once('+');
                    offsets.push(offset.map_or(Ok(0), |(_, n)| ld60_asm::parse_number(n.trim())));
                }
            }
        }
        offsets
            .into_iter()
            .flat_map(|offset| {
                let offset = offset.unwrap_or(0);
                (1..=max_b).map(move |b| ((offset + b) & 0x0F) as usize)
            })
            .collect()
    }

    /// Renames the RAM cells in code and comments.
    pub(crate) fn remap_ram(&mut self, map: &RamMap) {
        for text in &mut self.0 {
            *text = remap_cells(text, map);
        }
    }

    /// Swaps neighbouring instructions that do not depend on each other.
    pub(crate) fn swap_independent(&mut self, seed: u64) {
        let mut i = 0;
        while i + 1 < self.0.len() {
            let (first, second) = (Line::new(&self.0[i]), Line::new(&self.0[i + 1]));
            let independent = second.label().is_none()
                && Effects::of(&first)
                    .zip(Effects::of(&second))
                    .is_some_and(|(a, b)| a.commutes(b));
            let salt = format!("{}\n{}", first.rest, second.rest);
            if !independent || Rng::new(seed, &salt).next_u64() & 1 == 0 {
                i += 1;
                continue;
            }
            let swapped = [
                format!("{}{}", first.prefix, second.rest),
                format!("{}{}", second.prefix, first.rest),
            ];
            self.0[i..i + 2].clone_from_slice(&swapped);
            i += 2;
        }
    }

    /// Shuffles runs of `CPI n` and `JZ label` pairs, like the key decoder.
    // ponytail: C after the run changes. The test run in `Variant::new` catches code that uses it.
    pub(crate) fn shuffle_key_checks(&mut self, seed: u64) {
        let lines: Vec<Line<'_>> = self.lines().collect();
        let is_check = |i: usize| {
            let pair = lines.get(i).zip(lines.get(i + 1));
            pair.is_some_and(|(cpi, jz)| {
                cpi.label().is_none()
                    && jz.label().is_none()
                    && cpi.mnemonic().as_deref() == Some("CPI")
                    && jz.mnemonic().as_deref() == Some("JZ")
            })
        };
        let runs: Vec<(usize, usize)> = {
            let mut runs = Vec::new();
            let mut i = 0;
            while i < lines.len() {
                let len = (0..).take_while(|n| is_check(i + 2 * n)).count();
                if len >= 2 {
                    runs.push((i, len));
                }
                i += (2 * len).max(1);
            }
            runs
        };
        for (start, len) in runs {
            let range = start..start + 2 * len;
            let mut pairs: Vec<&[String]> = self.0[range.clone()].chunks(2).collect();
            Rng::new(seed, &self.0[range.clone()].join("\n")).shuffle(&mut pairs);
            let shuffled = pairs.concat();
            self.0[range].clone_from_slice(&shuffled);
        }
    }

    /// Moves the routines that end in `JMP` or `RET` around. The first one stays at address 0.
    pub(crate) fn reorder_routines(&mut self, seed: u64) {
        let has_directive = self
            .lines()
            .any(|line| line.mnemonic().is_some_and(|m| m.starts_with('.')));
        if has_directive {
            return;
        }
        let mut routines = Vec::new();
        let mut current = Vec::new();
        for text in self.0.drain(..) {
            let ends = matches!(Line::new(&text).mnemonic().as_deref(), Some("JMP" | "RET"));
            current.push(text);
            if ends {
                routines.push(std::mem::take(&mut current));
            }
        }
        if let Some(movable) = routines.get_mut(1..) {
            movable.sort_by_cached_key(|routine| {
                let label = routine.iter().find_map(|text| Line::new(text).label());
                Rng::new(seed, label.unwrap_or_default()).next_u64()
            });
            for routine in movable {
                if !routine.first().is_some_and(|text| text.trim().is_empty()) {
                    routine.insert(0, String::new());
                }
            }
        }
        self.0 = routines.concat();
        self.0.extend(current);
    }
}

impl fmt::Display for Firmware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|line| writeln!(f, "{line}"))
    }
}

/// Replaces each `Mn` token that stands alone.
fn remap_cells(text: &str, map: &RamMap) -> String {
    let bytes = text.as_bytes();
    let is_word = |i: usize| {
        bytes
            .get(i)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
    };
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    for i in 0..bytes.len() {
        let token = text.get(i..i + 2).and_then(super::line::cell);
        let alone = (i == 0 || !is_word(i - 1)) && !is_word(i + 2);
        if let Some(cell) = token.filter(|_| alone) {
            out.push_str(&text[copied..i]);
            out.push_str(&format!("M{:X}", map.get(cell)));
            copied = i + 2;
        }
    }
    out.push_str(&text[copied..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaps_only_whole_cell_tokens() {
        let pinned = (1..16).filter(|&c| c != 7);
        let map = (0..)
            .map(|seed| RamMap::shuffle(pinned.clone(), &mut Rng::new(seed, "")))
            .find(|map| map.get(0) == 7)
            .unwrap();
        assert_eq!(
            remap_cells("ST M0 ; M0 is MAX, not M0x or M7", &map),
            "ST M7 ; M7 is MAX, not M0x or M0"
        );
    }

    #[test]
    fn keeps_first_routine_and_moves_whole_routines() {
        let source = "start: JMP a\na: RET\nb: RET\nc: RET\nd: RET\n";
        let mut firmware = Firmware::new(source);
        firmware.reorder_routines(3);
        let text = firmware.to_string();
        assert!(text.starts_with("start: JMP a\n"));
        let mut lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        lines.sort_unstable();
        let mut expected: Vec<&str> = source.lines().collect();
        expected.sort_unstable();
        assert_eq!(lines, expected);
    }
}
