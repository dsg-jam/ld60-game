use std::ops::Range;

use crate::parse_number;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Number(i64),
    Name(String),
    Indirect { base: String, offset: i64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    /// Always upper case.
    pub mnemonic: String,
    pub operands: Vec<Operand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Directive {
    Org(i64),
    Db(Vec<i64>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Body {
    Instruction(Instruction),
    Directive(Directive),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Line {
    pub label: Option<String>,
    pub body: Option<Body>,
    pub spans: Spans,
}

/// Byte ranges of the parts of a [`Line`]. Missing parts are empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Spans {
    pub label: Range<usize>,
    /// The mnemonic or the directive with its dot.
    pub head: Range<usize>,
    pub operands: Range<usize>,
}

impl Spans {
    /// From the start of the head to the end of the operands.
    pub(crate) fn body(&self) -> Range<usize> {
        self.head.start..self.operands.end.max(self.head.end)
    }

    /// The operands, or the head if there are none.
    pub(crate) fn operands_or_head(&self) -> Range<usize> {
        if self.operands.is_empty() {
            self.head.clone()
        } else {
            self.operands.clone()
        }
    }
}

/// A parse error and the byte range of the line it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fault {
    pub message: String,
    pub span: Range<usize>,
}

pub(crate) fn parse_line(text: &str) -> Result<Line, Fault> {
    let span = |part: &str| {
        let start = part.as_ptr().addr() - text.as_ptr().addr();
        start..start + part.len()
    };
    let fault = |message: String, part: &str| Fault {
        message,
        span: span(part),
    };
    let code = text.split(';').next().unwrap_or_default().trim();
    let mut spans = Spans::default();
    let (label, rest) = match code.split_once(':') {
        Some((name, rest)) => {
            let name = name.trim();
            if !is_identifier(name) {
                return Err(fault(format!("invalid label '{name}'"), name));
            }
            spans.label = span(name);
            (Some(name.to_string()), rest.trim())
        }
        None => (None, code),
    };
    if rest.is_empty() {
        return Ok(Line {
            label,
            body: None,
            spans,
        });
    }
    let (head, tail) = match rest.split_once(char::is_whitespace) {
        Some((head, tail)) => (head, tail.trim()),
        None => (rest, &rest[rest.len()..]),
    };
    spans.head = span(head);
    spans.operands = span(tail);
    let operands = tail
        .split(',')
        .filter(|_| !tail.is_empty())
        .map(|part| {
            let part = part.trim();
            parse_operand(part).map_err(|message| {
                // An empty operand has no text to point at.
                fault(message, if part.is_empty() { tail } else { part })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let body = if let Some(directive) = head.strip_prefix('.') {
        let directive = parse_directive(directive, operands).map_err(|message| Fault {
            message,
            span: spans.operands_or_head(),
        })?;
        Body::Directive(directive)
    } else {
        Body::Instruction(Instruction {
            mnemonic: head.to_ascii_uppercase(),
            operands,
        })
    };
    Ok(Line {
        label,
        body: Some(body),
        spans,
    })
}

fn parse_directive(name: &str, operands: Vec<Operand>) -> Result<Directive, String> {
    let numbers = operands
        .into_iter()
        .map(|op| match op {
            Operand::Number(n) => Ok(n),
            _ => Err(format!(".{name} expects numbers")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    match name.to_ascii_lowercase().as_str() {
        "org" => match numbers.as_slice() {
            [addr] => Ok(Directive::Org(*addr)),
            _ => Err(".org expects one address".into()),
        },
        "db" if !numbers.is_empty() => Ok(Directive::Db(numbers)),
        "db" => Err(".db expects at least one value".into()),
        _ => Err(format!("unknown directive '.{name}'")),
    }
}

fn parse_operand(text: &str) -> Result<Operand, String> {
    if let Some(inner) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        let (base, offset) = match inner.split_once('+') {
            Some((base, offset)) => (base.trim(), parse_number(offset.trim())?),
            None => (inner.trim(), 0),
        };
        if !is_identifier(base) {
            return Err(format!("invalid operand '{text}'"));
        }
        return Ok(Operand::Indirect {
            base: base.to_string(),
            offset,
        });
    }
    if text.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
        return parse_number(text).map(Operand::Number);
    }
    if is_identifier(text) {
        return Ok(Operand::Name(text.to_string()));
    }
    Err(format!("invalid operand '{text}'"))
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instruction(text: &str) -> Instruction {
        match parse_line(text).unwrap().body {
            Some(Body::Instruction(i)) => i,
            other => panic!("expected instruction, got {other:?}"),
        }
    }

    fn fault_span(text: &str) -> Range<usize> {
        parse_line(text).unwrap_err().span
    }

    #[test]
    fn parses_label_mnemonic_and_operands() {
        let line = parse_line("loop:  ld [B+8]   ; total digit").unwrap();
        assert_eq!(line.label.as_deref(), Some("loop"));
        assert_eq!(
            line.body,
            Some(Body::Instruction(Instruction {
                mnemonic: "LD".into(),
                operands: vec![Operand::Indirect {
                    base: "B".into(),
                    offset: 8
                }],
            }))
        );
        assert_eq!(
            line.spans,
            Spans {
                label: 0..4,
                head: 7..9,
                operands: 10..15,
            }
        );
    }

    #[test]
    fn parses_multiple_operands() {
        let i = instruction("LDI A, 0x0A");
        assert_eq!(
            i.operands,
            vec![Operand::Name("A".into()), Operand::Number(10)]
        );
    }

    #[test]
    fn parses_directives() {
        let line = parse_line(".org 0x40").unwrap();
        assert_eq!(line.body, Some(Body::Directive(Directive::Org(0x40))));
        let line = parse_line(".db 1, 2, 0xFF").unwrap();
        assert_eq!(
            line.body,
            Some(Body::Directive(Directive::Db(vec![1, 2, 255])))
        );
    }

    #[test]
    fn empty_and_comment_lines_have_no_body() {
        assert_eq!(parse_line("").unwrap(), Line::default());
        assert_eq!(parse_line("   ; note").unwrap(), Line::default());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_line("1x: NOP").is_err());
        assert!(parse_line("LD [8+B]").is_err());
        assert!(parse_line("LDI A,").is_err());
        assert!(parse_line(".org").is_err());
        assert!(parse_line(".fill 3").is_err());
    }

    #[test]
    fn points_at_the_bad_part() {
        assert_eq!(fault_span(" 1x: NOP"), 1..3);
        assert_eq!(fault_span("LD A, 12a ; x"), 6..9);
        assert_eq!(fault_span("LDI A,"), 4..6);
        assert_eq!(fault_span("  .org"), 2..6);
        assert_eq!(fault_span(".org 1, 2"), 5..9);
    }
}
