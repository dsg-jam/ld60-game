use std::collections::HashSet;
use std::ops::Range;

use ld60_machine::Isa;

/// What a piece of source text is, for highlighting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Comment,
    Directive,
    /// A label definition, a reference to one, or an unknown name.
    Label,
    Mnemonic,
    Number,
    Port,
    Punctuation,
    Register,
    /// Whitespace and anything the lexer does not know.
    Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Token {
    /// Byte range in the line.
    pub(super) range: Range<usize>,
    pub(super) kind: Kind,
}

/// The names an ISA and a source give meaning to. The syntax around them
/// is the same for every ISA.
#[derive(Clone, Debug, Default)]
pub(super) struct Lexicon {
    mnemonics: &'static [&'static str],
    ports: &'static [&'static str],
    registers: &'static [&'static str],
    /// Labels defined in the source. They win over registers and ports.
    labels: HashSet<String>,
}

impl Lexicon {
    pub(super) fn new(isa: &dyn Isa, source: &str) -> Self {
        let labels = source
            .lines()
            .filter_map(|line| line.split(';').next()?.split_once(':'))
            .map(|(name, _)| name.trim().to_owned())
            .filter(|name| is_identifier(name))
            .collect();
        Self {
            mnemonics: isa.mnemonics(),
            ports: isa.ports(),
            registers: isa.registers(),
            labels,
        }
    }

    /// Splits one line of source into tokens. Whitespace between tokens is
    /// left out.
    pub(super) fn tokens(&self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let (code, comment) = match line.find(';') {
            Some(start) => (&line[..start], Some(start..line.len())),
            None => (line, None),
        };
        let mut rest = 0;
        if let Some(colon) = code.find(':') {
            push_word(&mut tokens, code, 0..colon, Kind::Label);
            tokens.push(Token {
                range: colon..colon + 1,
                kind: Kind::Punctuation,
            });
            rest = colon + 1;
        }
        let head_start = rest + leading_whitespace(&code[rest..]);
        let head_end = code[head_start..]
            .find(char::is_whitespace)
            .map_or(code.len(), |end| head_start + end);
        let head = &code[head_start..head_end];
        let kind = if head.starts_with('.') {
            Kind::Directive
        } else if contains(self.mnemonics, head) {
            Kind::Mnemonic
        } else {
            Kind::Text
        };
        push_word(&mut tokens, code, head_start..head_end, kind);
        self.operands(&mut tokens, code, head_end);
        if let Some(range) = comment {
            tokens.push(Token {
                range,
                kind: Kind::Comment,
            });
        }
        tokens
    }

    fn operands(&self, tokens: &mut Vec<Token>, code: &str, start: usize) {
        let mut word = None;
        for (index, c) in code[start..].char_indices() {
            let index = start + index;
            let separator = matches!(c, ',' | '[' | ']' | '+');
            if separator || c.is_whitespace() {
                if let Some(word_start) = word.take() {
                    self.push_operand(tokens, code, word_start..index);
                }
                if separator {
                    tokens.push(Token {
                        range: index..index + c.len_utf8(),
                        kind: Kind::Punctuation,
                    });
                }
            } else if word.is_none() {
                word = Some(index);
            }
        }
        if let Some(word_start) = word {
            self.push_operand(tokens, code, word_start..code.len());
        }
    }

    fn push_operand(&self, tokens: &mut Vec<Token>, code: &str, range: Range<usize>) {
        let text = &code[range.clone()];
        let kind = if text.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
            Kind::Number
        } else if !is_identifier(text) {
            Kind::Text
        } else if self.labels.contains(text) {
            Kind::Label
        } else if contains(self.registers, text) {
            Kind::Register
        } else if contains(self.ports, text) {
            Kind::Port
        } else {
            Kind::Label
        };
        tokens.push(Token { range, kind });
    }
}

/// Pushes the text in `range` without its surrounding whitespace.
fn push_word(tokens: &mut Vec<Token>, code: &str, range: Range<usize>, kind: Kind) {
    let text = &code[range.clone()];
    let start = range.start + leading_whitespace(text);
    let end = range.start + text.trim_end().len();
    if start < end {
        tokens.push(Token {
            range: start..end,
            kind,
        });
    }
}

fn leading_whitespace(text: &str) -> usize {
    text.len() - text.trim_start().len()
}

fn contains(names: &[&str], text: &str) -> bool {
    names.iter().any(|name| name.eq_ignore_ascii_case(text))
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

    fn toy() -> Lexicon {
        Lexicon {
            mnemonics: &["LDI", "JMP", "NOP", "IN"],
            ports: &["KEY"],
            registers: &["A", "B"],
            labels: HashSet::new(),
        }
    }

    fn kinds(line: &str) -> Vec<(&str, Kind)> {
        toy()
            .tokens(line)
            .into_iter()
            .map(|token| (&line[token.range], token.kind))
            .collect()
    }

    #[test]
    fn splits_a_full_line() {
        use Kind::*;
        assert_eq!(
            kinds("loop: LDI A, 0x0D ; jmp"),
            vec![
                ("loop", Label),
                (":", Punctuation),
                ("LDI", Mnemonic),
                ("A", Register),
                (",", Punctuation),
                ("0x0D", Number),
                ("; jmp", Comment),
            ]
        );
    }

    #[test]
    fn labels_may_look_like_mnemonics() {
        use Kind::*;
        assert_eq!(
            kinds("jmp:  jmp nop"),
            vec![
                ("jmp", Label),
                (":", Punctuation),
                ("jmp", Mnemonic),
                ("nop", Label)
            ]
        );
    }

    #[test]
    fn uses_only_the_names_of_the_isa() {
        use Kind::*;
        assert_eq!(kinds("CALL x"), vec![("CALL", Text), ("x", Label)]);
        assert_eq!(kinds("in key"), vec![("in", Mnemonic), ("key", Port)]);
        let calco = Lexicon::new(ld60_calco::Revision::C.isa(), "");
        let tokens = calco.tokens("CALL MF");
        assert_eq!(tokens[0].kind, Mnemonic);
        assert_eq!(tokens[1].kind, Register);
    }

    #[test]
    fn names_defined_as_labels_are_labels() {
        let calco = Lexicon::new(
            ld60_calco::Revision::C.isa(),
            "a: NOP\n key:  ; x\nb ; c: NOP",
        );
        let kinds: Vec<_> = ["JMP a", "JMP key", "JMP b", "IN KEY"]
            .into_iter()
            .map(|line| calco.tokens(line)[1].kind)
            .collect();
        assert_eq!(
            kinds,
            vec![Kind::Label, Kind::Label, Kind::Register, Kind::Port]
        );
    }

    #[test]
    fn handles_directives_and_brackets() {
        use Kind::*;
        assert_eq!(
            kinds("  .org 0x40"),
            vec![(".org", Directive), ("0x40", Number)]
        );
        assert_eq!(
            kinds("LD [B+8"),
            vec![
                ("LD", Text),
                ("[", Punctuation),
                ("B", Register),
                ("+", Punctuation),
                ("8", Number)
            ]
        );
    }

    #[test]
    fn keeps_odd_input_as_text() {
        use Kind::*;
        assert_eq!(kinds(""), vec![]);
        assert_eq!(kinds("   "), vec![]);
        assert_eq!(kinds(";"), vec![(";", Comment)]);
        assert_eq!(kinds(":"), vec![(":", Punctuation)]);
        assert_eq!(
            kinds("NOP {fc},\u{e9}x ; \u{fc}"),
            vec![
                ("NOP", Mnemonic),
                ("{fc}", Text),
                (",", Punctuation),
                ("\u{e9}x", Text),
                ("; \u{fc}", Comment),
            ]
        );
        assert_eq!(
            kinds("a: b: NOP"),
            vec![
                ("a", Label),
                (":", Punctuation),
                ("b:", Text),
                ("NOP", Label)
            ]
        );
    }
}
