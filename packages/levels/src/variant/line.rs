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

/// Parses `M0` to `MF`.
pub(crate) fn cell(token: &str) -> Option<u8> {
    let digit = token.strip_prefix('M')?;
    let mut chars = digit.chars();
    let n = chars.next().filter(char::is_ascii_hexdigit)?.to_digit(16)?;
    chars.next().is_none().then_some(n as u8)
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
}
