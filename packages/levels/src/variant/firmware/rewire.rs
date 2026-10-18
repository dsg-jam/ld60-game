use ld60_calco::ports;
use ld60_devices::Board;

use super::Firmware;
use crate::variant::line::{Line, splice};
use crate::variant::ram_map::RamMap;

impl Firmware {
    /// Moves firmware written for the `from` board to the `to` board.
    ///
    /// Returns the cell map, or `None` if the firmware uses the I/O in a way
    /// the rewrite cannot follow.
    pub(crate) fn rewire(&mut self, from: &Board, to: &Board) -> Option<RamMap> {
        let by = to.display.wrapping_sub(from.display);
        let map = RamMap::rotate(by);
        self.remap_ram(&map);
        self.shift_indexed(by);
        self.remap_keys(from, to)?;
        self.remap_annunciators(from, to)?;
        Some(map)
    }

    /// Adds `by` to the offset of every `[B+n]`, so it follows the rotated cells.
    fn shift_indexed(&mut self, by: u8) {
        if by.is_multiple_of(16) {
            return;
        }
        for text in &mut self.0 {
            let line = Line::new(text);
            let indexed = line.operands().into_iter().find_map(|operand| {
                let inner = operand.strip_prefix('[')?.strip_suffix(']')?;
                let (register, offset) = match inner.split_once('+') {
                    Some((register, offset)) => (register.trim(), number(offset.trim())?),
                    None => (inner.trim(), 0),
                };
                let offset = offset.wrapping_add(by) % 16;
                let new = match offset {
                    0 => format!("[{register}]"),
                    _ => format!("[{register}+{offset}]"),
                };
                Some(splice(text, operand, &new))
            });
            if let Some(new) = indexed {
                *text = new;
            }
        }
    }

    /// Rewrites `CPI` immediates that test a key code read from `KEY`.
    ///
    /// Each compare keeps its outcome for every key: the same keys are equal
    /// and the same keys are lower.
    // ponytail: follows only straight code after `IN KEY`. The test run in `Variant::new` catches the rest.
    fn remap_keys(&mut self, from: &Board, to: &Board) -> Option<()> {
        if from.keys == to.keys {
            return Some(());
        }
        let lines: Vec<Line<'_>> = self.lines().collect();
        let mut edits = Vec::new();
        let mut reading = false;
        for (i, line) in lines.iter().enumerate() {
            if line.label().is_some() {
                reading = false;
            }
            let Some(mnemonic) = line.mnemonic() else {
                continue;
            };
            let operands = line.operands();
            if mnemonic == "IN" && is_port(&operands, "KEY", ports::KEY) {
                reading = true;
                continue;
            }
            if !reading {
                continue;
            }
            match (mnemonic.as_str(), operands.as_slice()) {
                ("CPI", [value]) => {
                    let next = lines[i + 1..].iter().find_map(Line::mnemonic);
                    let (zero, carry) = match next.as_deref() {
                        Some("JZ" | "JNZ") => (true, false),
                        Some("JC" | "JNC") => (false, true),
                        _ => (true, true),
                    };
                    let old = number(value)?;
                    let new = key_compare(from, to, old, zero, carry)?;
                    if new != old {
                        edits.push((i, splice(&self.0[i], value, &immediate(value, new))));
                    }
                }
                ("LDI", ["B" | "b", _])
                | (
                    "ST" | "JZ" | "JNZ" | "JC" | "JNC" | "DJNZ" | "NOP" | "CLC" | "SEC" | "HLT",
                    _,
                ) => {}
                ("LD" | "LDI" | "IN" | "JMP" | "RET", _) => reading = false,
                _ => return None,
            }
        }
        for (i, text) in edits {
            self.0[i] = text;
        }
        Some(())
    }

    /// Rewrites the `LDI A, n` in front of each `OUT ANN` to light the same
    /// annunciators.
    fn remap_annunciators(&mut self, from: &Board, to: &Board) -> Option<()> {
        if from.annunciators == to.annunciators {
            return Some(());
        }
        let lines: Vec<Line<'_>> = self.lines().collect();
        let code: Vec<usize> = (0..lines.len())
            .filter(|&i| lines[i].mnemonic().is_some())
            .collect();
        let mut edits = Vec::new();
        for (k, &i) in code.iter().enumerate() {
            let is_out = lines[i].mnemonic().as_deref() == Some("OUT")
                && is_port(&lines[i].operands(), "ANN", ports::ANN);
            if !is_out {
                continue;
            }
            let load = code[..k].last().copied()?;
            if lines[load + 1..=i]
                .iter()
                .any(|line| line.label().is_some())
            {
                return None;
            }
            let value = match lines[load].operands().as_slice() {
                ["A" | "a", value] if lines[load].mnemonic().as_deref() == Some("LDI") => *value,
                _ => return None,
            };
            let old = number(value)?;
            let new = annunciator_bits(from, to, old)?;
            if new == old {
                continue;
            }
            let after = code.get(k + 1).and_then(|&n| lines[n].mnemonic());
            let overwritten = match after.as_deref() {
                None | Some("RET" | "JMP" | "HLT" | "LD" | "IN") => true,
                Some("LDI") => lines[code[k + 1]]
                    .operands()
                    .first()
                    .is_some_and(|register| register.eq_ignore_ascii_case("A")),
                Some(_) => false,
            };
            if !overwritten {
                return None;
            }
            edits.push((load, splice(&self.0[load], value, &immediate(value, new))));
        }
        for (i, text) in edits {
            self.0[i] = text;
        }
        Some(())
    }
}

/// Moves each bit of an `ANN` value to the bit of the same annunciator on `to`.
fn annunciator_bits(from: &Board, to: &Board, old: u8) -> Option<u8> {
    (0..8)
        .filter(|bit| old >> bit & 1 != 0)
        .try_fold(0, |new, bit| {
            let &(annunciator, _) = from.annunciators.iter().find(|(_, b)| *b == bit)?;
            Some(new | 1 << to.ann_bit(annunciator)?)
        })
}

/// The `CPI` immediate on `to` that gives every key the same flags as `old` on `from`.
///
/// A test for equal keeps to its key. A test for lower keeps its bound if it can.
fn key_compare(from: &Board, to: &Board, old: u8, zero: bool, carry: bool) -> Option<u8> {
    let moved = from
        .keys
        .iter()
        .find(|&&(_, code)| code == old)
        .and_then(|&(key, _)| to.key_code(key));
    let candidates = if zero {
        [moved, Some(old)]
    } else {
        [Some(old), moved]
    };
    candidates.into_iter().flatten().find(|&new| {
        from.keys.iter().all(|&(key, code)| {
            to.key_code(key).is_some_and(|moved| {
                (!zero || (code == old) == (moved == new))
                    && (!carry || (code < old) == (moved < new))
            })
        })
    })
}

/// Writes `value` in the style of `original`, hex or decimal.
fn immediate(original: &str, value: u8) -> String {
    if original.starts_with("0x") || original.starts_with("0X") {
        format!("0x{value:02X}")
    } else {
        value.to_string()
    }
}

fn is_port(operands: &[&str], name: &str, port: u8) -> bool {
    match operands {
        [operand] => operand.eq_ignore_ascii_case(name) || number(operand) == Some(port),
        _ => false,
    }
}

fn number(text: &str) -> Option<u8> {
    u8::try_from(ld60_asm::parse_number(text).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use ld60_devices::{BoardRev, DeviceKind};

    use super::*;
    use crate::CATALOG;

    fn rewire(source: &str, kind: DeviceKind, to: BoardRev) -> Option<String> {
        let mut firmware = Firmware::new(source);
        firmware.rewire(kind.board(BoardRev::A), kind.board(to))?;
        Some(firmware.to_string())
    }

    #[test]
    fn same_board_changes_nothing() {
        for source in CATALOG {
            let device = source.load().unwrap().device;
            for text in [source.buggy, source.fixed] {
                let expected = Firmware::new(text).to_string();
                assert_eq!(rewire(text, device, BoardRev::A).unwrap(), expected);
            }
        }
    }

    #[test]
    fn moves_cells_keys_and_annunciators() {
        let source = "\
; M1-M6 display, M6 is the rightmost digit
main:   HLT
        IN   KEY
        CPI  10
        JC   digit
        CPI  0x0A
        JZ   plus
        JMP  main
digit:  ST   [B+8]
        LD   [B]
        CPI  10
        JMP  main
plus:   LDI  A, 1
        OUT  ANN
        RET
";
        let expected = "\
; M2-M7 display, M7 is the rightmost digit
main:   HLT
        IN   KEY
        CPI  10
        JC   digit
        CPI  0x0C
        JZ   plus
        JMP  main
digit:  ST   [B+9]
        LD   [B+1]
        CPI  10
        JMP  main
plus:   LDI  A, 2
        OUT  ANN
        RET
";
        let rewired = rewire(source, DeviceKind::Summa6, BoardRev::B);
        assert_eq!(rewired.as_deref(), Some(expected));
    }

    #[test]
    fn keeps_a_bound_that_matches_a_key_code() {
        let source = "IN KEY\nCPI 10\nJC digit\nCPI 0x0A\nJZ plus\n";
        let rewired = rewire(source, DeviceKind::Summa6, BoardRev::C).unwrap();
        assert_eq!(rewired, "IN KEY\nCPI 10\nJC digit\nCPI 0x0B\nJZ plus\n");
    }

    #[test]
    fn rejects_io_it_cannot_follow() {
        let unknown_ann = "LD M1\nOUT ANN\n";
        assert_eq!(rewire(unknown_ann, DeviceKind::Summa6, BoardRev::B), None);
        let key_math = "IN KEY\nDEC\nJZ one\n";
        assert_eq!(rewire(key_math, DeviceKind::Tally2, BoardRev::B), None);
        let ann_reused = "LDI A, 1\nOUT ANN\nST M7\n";
        assert_eq!(rewire(ann_reused, DeviceKind::Summa6, BoardRev::B), None);
    }
}
