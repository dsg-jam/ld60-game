/// A source line split into the label part and the rest.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Line<'a> {
    /// The label with its padding, or the indentation.
    pub(crate) prefix: &'a str,
    /// The instruction and the comment.
    pub(crate) rest: &'a str,
}

impl<'a> Line<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        let code = text.split(';').next().unwrap_or_default();
        let start = code.find(':').map_or(0, |colon| colon + 1);
        let split = text.len() - text[start..].trim_start().len();
        let (prefix, rest) = text.split_at(split);
        Self { prefix, rest }
    }

    fn code(&self) -> &'a str {
        self.rest.split(';').next().unwrap_or_default().trim()
    }

    pub(crate) fn label(&self) -> Option<&'a str> {
        self.prefix.split_once(':').map(|(name, _)| name.trim())
    }

    pub(crate) fn mnemonic(&self) -> Option<String> {
        self.code()
            .split_whitespace()
            .next()
            .map(str::to_ascii_uppercase)
    }

    pub(crate) fn operands(&self) -> Vec<&'a str> {
        match self.code().split_once(char::is_whitespace) {
            Some((_, tail)) => tail.split(',').map(str::trim).collect(),
            None => Vec::new(),
        }
    }
}

/// Parses `M0` to `MF`, in any case like the assembler.
pub(crate) fn cell(token: &str) -> Option<u8> {
    let digit = token.strip_prefix(['M', 'm'])?;
    let mut chars = digit.chars();
    let n = chars.next().filter(char::is_ascii_hexdigit)?.to_digit(16)?;
    chars.next().is_none().then_some(n as u8)
}

/// Parses an immediate like `10` or `0x0A`.
pub(crate) fn number(text: &str) -> Option<u8> {
    u8::try_from(ld60_asm::parse_number(text).ok()?).ok()
}

/// Writes `value` in the style of `original`, hex or decimal.
pub(crate) fn immediate(original: &str, value: u8) -> String {
    if original.starts_with("0x") || original.starts_with("0X") {
        format!("0x{value:02X}")
    } else {
        value.to_string()
    }
}

/// Replaces `part`, which must be a slice of `text`, with `new`.
pub(crate) fn splice(text: &str, part: &str, new: &str) -> String {
    let start = part.as_ptr().addr() - text.as_ptr().addr();
    format!("{}{new}{}", &text[..start], &text[start + part.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_label_instruction_and_comment() {
        let line = Line::new("loop:  ld [B+8]   ; total: digit");
        assert_eq!(line.prefix, "loop:  ");
        assert_eq!(line.label(), Some("loop"));
        assert_eq!(line.mnemonic().as_deref(), Some("LD"));
        assert_eq!(line.operands(), ["[B+8]"]);

        let line = Line::new("        LDI  A, 0");
        assert_eq!(line.label(), None);
        assert_eq!(line.operands(), ["A", "0"]);

        assert_eq!(Line::new("; note: x").mnemonic(), None);
    }

    #[test]
    fn parses_cells_in_any_case() {
        assert_eq!(cell("MA"), Some(10));
        assert_eq!(cell("m4"), Some(4));
        assert_eq!(cell("MG"), None);
        assert_eq!(cell("M10"), None);
    }
}
