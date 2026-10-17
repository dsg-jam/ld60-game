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
}

pub(crate) fn parse_line(text: &str) -> Result<Line, String> {
    let code = text.split(';').next().unwrap_or_default().trim();
    let (label, rest) = match code.split_once(':') {
        Some((name, rest)) => {
            let name = name.trim();
            if !is_identifier(name) {
                return Err(format!("invalid label '{name}'"));
            }
            (Some(name.to_string()), rest.trim())
        }
        None => (None, code),
    };
    if rest.is_empty() {
        return Ok(Line { label, body: None });
    }
    let (head, tail) = match rest.split_once(char::is_whitespace) {
        Some((head, tail)) => (head, tail.trim()),
        None => (rest, ""),
    };
    let operands = parse_operands(tail)?;
    let body = if let Some(directive) = head.strip_prefix('.') {
        Body::Directive(parse_directive(directive, operands)?)
    } else {
        Body::Instruction(Instruction {
            mnemonic: head.to_ascii_uppercase(),
            operands,
        })
    };
    Ok(Line {
        label,
        body: Some(body),
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

fn parse_operands(text: &str) -> Result<Vec<Operand>, String> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    text.split(',')
        .map(|part| parse_operand(part.trim()))
        .collect()
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
}
