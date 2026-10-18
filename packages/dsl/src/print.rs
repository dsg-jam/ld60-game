use std::fmt::Write;

use crate::emit::{Instr, Line, Operand};
use crate::hir::Program;
use crate::labels::Names;

const CODE_COLUMN: usize = 8;
const COMMENT_COLUMN: usize = 28;

/// Prints the header and the code in the layout of the hand-written files.
pub(crate) fn print(program: &Program, lines: &[Line], names: &Names) -> String {
    let mut out = format!("; {}\n;\n", program.title);
    for ram in &program.ram {
        let cells = if ram.first == ram.last {
            format!("M{:X}", ram.first)
        } else {
            format!("M{:X}-M{:X}", ram.first, ram.last)
        };
        let _ = writeln!(out, "; {cells:<6} {}", ram.text);
    }
    if !program.notes.is_empty() {
        out.push_str(";\n");
        for note in &program.notes {
            out.push_str(&comment_line(note));
            out.push('\n');
        }
    }
    out.push('\n');

    let mut label: Option<String> = None;
    for line in lines {
        let text = match line {
            Line::Label(id) => {
                if names.is_shown(*id)
                    && let Some(previous) = label.replace(format!("{}:", names.get(*id)))
                {
                    out.push_str(&previous);
                    out.push('\n');
                }
                continue;
            }
            Line::Blank => String::new(),
            Line::Comment(text) => format!("{}{}", " ".repeat(CODE_COLUMN), comment_line(text)),
            Line::Instr(instr) => code(
                label.take(),
                &instruction(instr, names),
                instr.comment.as_deref(),
            ),
            Line::Raw(raw) => raw_line(label.take(), raw),
        };
        if let (Line::Blank | Line::Comment(_), Some(pending)) = (line, label.take()) {
            out.push_str(&pending);
            out.push('\n');
        }
        out.push_str(text.trim_end());
        out.push('\n');
    }
    if let Some(pending) = label {
        out.push_str(&pending);
        out.push('\n');
    }
    out
}

fn code(label: Option<String>, body: &str, comment: Option<&str>) -> String {
    let mut text = String::new();
    if let Some(label) = label {
        if label.len() >= CODE_COLUMN {
            text.push_str(&label);
            text.push('\n');
        } else {
            text.push_str(&label);
        }
    }
    let line_start = text.rfind('\n').map_or(0, |i| i + 1);
    let pad = CODE_COLUMN.saturating_sub(text.len() - line_start);
    text.push_str(&" ".repeat(pad));
    text.push_str(body);
    if let Some(comment) = comment {
        let used = text.len() - line_start;
        let pad = COMMENT_COLUMN.saturating_sub(used).max(1);
        text.push_str(&" ".repeat(pad));
        text.push_str(&comment_line(comment));
    }
    text
}

fn comment_line(text: &str) -> String {
    if text.is_empty() {
        ";".into()
    } else {
        format!("; {text}")
    }
}

fn instruction(instr: &Instr, names: &Names) -> String {
    let operands: Vec<&str> = instr
        .operands
        .iter()
        .map(|operand| match operand {
            Operand::Label(id) => names.get(*id),
            Operand::Text(text) => text.as_str(),
        })
        .collect();
    if operands.is_empty() {
        instr.mnemonic.into()
    } else {
        format!("{:<4} {}", instr.mnemonic, operands.join(", "))
    }
}

/// An `asm { }` line. A leading `label:` goes in the label column.
fn raw_line(label: Option<String>, raw: &str) -> String {
    let code_part = raw.split(';').next().unwrap_or_default();
    if let Some((name, _)) = code_part.split_once(':') {
        let name = name.trim();
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            let rest = raw[name.len() + 1..].trim();
            let mut text = String::new();
            if let Some(label) = label {
                text.push_str(&label);
                text.push('\n');
            }
            text.push_str(&format!("{name}:"));
            if !rest.is_empty() {
                let pad = CODE_COLUMN.saturating_sub(name.len() + 1).max(1);
                text.push_str(&" ".repeat(pad));
                text.push_str(rest);
            }
            return text;
        }
    }
    code(label, raw, None)
}
