use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use machine::{AsmError, Assembly, Image, SourceLine};

use crate::Encoder;
use crate::Labels;
use crate::parse::{Body, Directive, parse_line};

struct Placed {
    line: usize,
    addr: usize,
    body: Body,
}

pub fn assemble(source: &str, encoder: &dyn Encoder) -> Result<Assembly, Vec<AsmError>> {
    let mut errors = Vec::new();
    let (placed, labels) = place(source, encoder, &mut errors);
    let (image, lines) = emit(&placed, &labels, encoder, &mut errors);
    if errors.is_empty() {
        Ok(Assembly {
            image,
            lines,
            labels,
        })
    } else {
        errors.sort_by_key(|e| e.line);
        Err(errors)
    }
}

fn place(
    source: &str,
    encoder: &dyn Encoder,
    errors: &mut Vec<AsmError>,
) -> (Vec<Placed>, BTreeMap<String, u16>) {
    let mut placed = Vec::new();
    let mut labels = BTreeMap::new();
    let mut addr = 0usize;
    for (index, text) in source.lines().enumerate() {
        let line = index + 1;
        let mut fail = |message: String| errors.push(AsmError { line, message });
        let parsed = match parse_line(text) {
            Ok(parsed) => parsed,
            Err(message) => {
                fail(message);
                continue;
            }
        };
        if let Some(name) = parsed.label {
            match labels.entry(name) {
                Entry::Vacant(e) => {
                    e.insert(u16::try_from(addr).unwrap_or(u16::MAX));
                }
                Entry::Occupied(e) => fail(format!("duplicate label '{}'", e.key())),
            }
        }
        let Some(body) = parsed.body else { continue };
        let size = match &body {
            Body::Directive(Directive::Org(target)) => {
                match usize::try_from(*target) {
                    Ok(target) if target <= encoder.rom_len() => addr = target,
                    _ => fail(format!("address {target} is outside the ROM")),
                }
                continue;
            }
            Body::Directive(Directive::Db(values)) => values.len(),
            Body::Instruction(instruction) => match encoder.size(instruction) {
                Ok(size) => size,
                Err(message) => {
                    fail(message);
                    continue;
                }
            },
        };
        placed.push(Placed { line, addr, body });
        addr += size;
    }
    (placed, labels)
}

fn emit(
    placed: &[Placed],
    labels: &BTreeMap<String, u16>,
    encoder: &dyn Encoder,
    errors: &mut Vec<AsmError>,
) -> (Image, Vec<SourceLine>) {
    let mut image = Image::blank(encoder.rom_len(), encoder.word_bits());
    let mut used = vec![false; image.len()];
    let mut lines = Vec::new();
    let labels = Labels::new(labels);
    for item in placed {
        let mut fail = |message: String| {
            errors.push(AsmError {
                line: item.line,
                message,
            })
        };
        let words = match &item.body {
            Body::Directive(Directive::Db(values)) => values
                .iter()
                .map(|&v| word(v, image.mask()))
                .collect::<Result<Vec<_>, _>>(),
            Body::Directive(Directive::Org(_)) => Ok(Vec::new()),
            Body::Instruction(instruction) => encoder.encode(instruction, &labels),
        };
        let words = match words {
            Ok(words) => words,
            Err(message) => {
                fail(message);
                continue;
            }
        };
        let end = item.addr + words.len();
        if end > image.len() {
            fail("code does not fit in the ROM".into());
            continue;
        }
        if let Some(addr) = (item.addr..end).find(|&a| used[a]) {
            fail(format!("overlaps code at 0x{addr:02X}"));
            continue;
        }
        for (offset, &w) in words.iter().enumerate() {
            image.set(item.addr + offset, w);
            used[item.addr + offset] = true;
        }
        lines.push(SourceLine {
            line: item.line,
            addr: u16::try_from(item.addr).unwrap_or(u16::MAX),
            len: u16::try_from(words.len()).unwrap_or(u16::MAX),
        });
    }
    (image, lines)
}

fn word(value: i64, mask: u16) -> Result<u16, String> {
    u16::try_from(value)
        .ok()
        .filter(|&w| w <= mask)
        .ok_or_else(|| format!("value {value} does not fit in a word"))
}
