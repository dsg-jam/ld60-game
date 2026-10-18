use crate::diagnostic::Pos;
use crate::emit::{Instr, Line, Operand};
use crate::hir::Program;
use crate::labels::{Names, asm_label};

const CODE_COLUMN: usize = 8;
const COMMENT_COLUMN: usize = 28;

/// The printed firmware and the source position of each output line.
pub(crate) struct Printed {
    pub(crate) text: String,
    /// Indexed by output line number minus 1.
    pub(crate) positions: Vec<Pos>,
}

impl Printed {
    fn push(&mut self, text: &str, pos: Pos) {
        for line in text.split('\n') {
            self.text.push_str(line.trim_end());
            self.text.push('\n');
            self.positions.push(pos);
        }
    }
}

/// Prints the header and the code in the layout of the hand-written files.
pub(crate) fn print(program: &Program, lines: &[Line], names: &Names) -> Printed {
    let mut out = Printed {
        text: String::new(),
        positions: Vec::new(),
    };
    let header = Pos::default();
    out.push(&format!("; {}\n;", program.title), header);
    for ram in &program.ram {
        let cells = if ram.first == ram.last {
            format!("M{:X}", ram.first)
        } else {
            format!("M{:X}-M{:X}", ram.first, ram.last)
        };
        out.push(&format!("; {cells:<6} {}", ram.text), header);
    }
    if !program.notes.is_empty() {
        out.push(";", header);
        for note in &program.notes {
            out.push(&comment_line(note), header);
        }
    }
    out.push("", header);

    let mut label: Option<(String, Pos)> = None;
    for line in lines {
        let (text, pos) = match line {
            Line::Label(id) => {
                if names.is_shown(*id)
                    && let Some((previous, pos)) =
                        label.replace((format!("{}:", names.get(*id)), Pos::default()))
                {
                    out.push(&previous, pos);
                }
                continue;
            }
            Line::Blank => (String::new(), Pos::default()),
            Line::Comment(text) => (
                format!("{}{}", " ".repeat(CODE_COLUMN), comment_line(text)),
                Pos::default(),
            ),
            Line::Instr(instr) => (
                code(
                    label.take().map(|(l, _)| l),
                    &instruction(instr, names),
                    instr.comment.as_deref(),
                ),
                instr.pos,
            ),
            Line::Raw(raw, pos) => (raw_line(label.take().map(|(l, _)| l), raw), *pos),
        };
        if let (Line::Blank | Line::Comment(_), Some((pending, pos))) = (line, label.take()) {
            out.push(&pending, pos);
        }
        out.push(&text, pos);
    }
    if let Some((pending, pos)) = label {
        out.push(&pending, pos);
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
    let Some((name, rest)) = asm_label(raw) else {
        return code(label, raw, None);
    };
    let mut text = String::new();
    if let Some(label) = label {
        text.push_str(&label);
        text.push('\n');
    }
    text.push_str(name);
    text.push(':');
    if !rest.is_empty() {
        let pad = CODE_COLUMN.saturating_sub(name.len() + 1).max(1);
        text.push_str(&" ".repeat(pad));
        text.push_str(rest);
    }
    text
}
