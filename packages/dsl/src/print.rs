use crate::backend::{Backend, Syntax};
use crate::diagnostic::Pos;
use crate::emit::{Line, Operand};
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

struct Printer<'a> {
    syntax: Syntax,
    names: &'a Names,
}

impl Printer<'_> {
    /// `body` in the code column, with an optional label before it and comment after it.
    fn code(&self, label: Option<String>, body: &str, comment: Option<&str>) -> String {
        let mut text = String::new();
        if let Some(label) = label {
            text.push_str(&label);
            if label.len() >= CODE_COLUMN {
                text.push('\n');
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
            text.push_str(&self.comment(comment));
        }
        text
    }

    fn comment(&self, text: &str) -> String {
        if text.is_empty() {
            self.syntax.comment.into()
        } else {
            format!("{} {text}", self.syntax.comment)
        }
    }

    fn label(&self, name: &str) -> String {
        format!("{name}{}", self.syntax.label_end)
    }

    /// An `asm { }` line. A leading label goes in the label column.
    fn raw(&self, label: Option<String>, raw: &str) -> String {
        let Some((name, rest)) = asm_label(raw, self.syntax) else {
            return self.code(label, raw, None);
        };
        let mut text = String::new();
        if let Some(label) = label {
            text.push_str(&label);
            text.push('\n');
        }
        let name = self.label(name);
        text.push_str(&name);
        if !rest.is_empty() {
            let pad = CODE_COLUMN.saturating_sub(name.len()).max(1);
            text.push_str(&" ".repeat(pad));
            text.push_str(rest);
        }
        text
    }
}

/// Prints the header and the code in the layout of the hand-written files.
pub(crate) fn print(
    program: &Program,
    lines: &[Line],
    names: &Names,
    backend: &dyn Backend,
) -> Printed {
    let printer = Printer {
        syntax: backend.syntax(),
        names,
    };
    let mut out = Printed {
        text: String::new(),
        positions: Vec::new(),
    };
    let header = Pos::default();
    out.push(&printer.comment(&program.title), header);
    out.push(&printer.comment(""), header);
    for ram in &program.ram {
        let cells = if ram.first == ram.last {
            backend.cell_name(ram.first)
        } else {
            let (first, last) = (backend.cell_name(ram.first), backend.cell_name(ram.last));
            format!("{first}-{last}")
        };
        out.push(
            &printer.comment(&format!("{cells:<6} {}", ram.text)),
            header,
        );
    }
    if !program.notes.is_empty() {
        out.push(&printer.comment(""), header);
        for note in &program.notes {
            out.push(&printer.comment(note), header);
        }
    }
    out.push("", header);

    let mut label: Option<(String, Pos)> = None;
    for line in lines {
        let (text, pos) = match line {
            Line::Label(id) => {
                if names.is_shown(*id)
                    && let Some((previous, pos)) =
                        label.replace((printer.label(names.get(*id)), Pos::default()))
                {
                    out.push(&previous, pos);
                }
                continue;
            }
            Line::Blank => (String::new(), Pos::default()),
            Line::Comment(text) => (
                format!("{}{}", " ".repeat(CODE_COLUMN), printer.comment(text)),
                Pos::default(),
            ),
            Line::Instr(instr) => {
                let operands: Vec<&str> = instr
                    .operands
                    .iter()
                    .map(|operand| match operand {
                        Operand::Label(id) => printer.names.get(*id),
                        Operand::Text(text) => text.as_str(),
                    })
                    .collect();
                let body = backend.format(instr, &operands);
                let pending = label.take().map(|(l, _)| l);
                (
                    printer.code(pending, &body, instr.comment.as_deref()),
                    instr.pos,
                )
            }
            Line::Raw(raw, pos) => (printer.raw(label.take().map(|(l, _)| l), raw), *pos),
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
