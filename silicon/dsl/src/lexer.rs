use std::iter::Peekable;
use std::str::Chars;

use crate::Diagnostic;
use crate::diagnostic::Pos;
use crate::literal::Literal;

const SYMBOLS: &[&str] = &[
    "=>", "==", "!=", ">=", "+=", "-=", "{", "}", "(", ")", "[", "]", "=", "<", "+", "-", ",", ":",
    "!",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Tok {
    /// Raw lines of an `asm { }` block.
    Asm(Vec<String>),
    /// A `;` comment, without the `;` and surrounding spaces.
    Comment(String),
    Eof,
    Ident(String),
    Key(char),
    Newline,
    Number(Literal),
    Str(String),
    Sym(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) tok: Tok,
    pub(crate) pos: Pos,
}

struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    pos: Pos,
}

impl Lexer<'_> {
    fn asm(&mut self, start: Pos) -> Result<Tok, Diagnostic> {
        self.skip_blanks();
        if self.chars.peek() != Some(&'{') {
            return Ok(Tok::Ident("asm".into()));
        }
        self.bump();
        let mut text = String::new();
        loop {
            match self.bump() {
                Some('}') => break,
                Some(c) => text.push(c),
                None => return Err(Diagnostic::error(start, "asm block is not closed")),
            }
        }
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(String::from)
            .collect();
        Ok(Tok::Asm(lines))
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        if c == '\n' {
            self.pos.line += 1;
            self.pos.col = 1;
        } else {
            self.pos.col += 1;
        }
        Some(c)
    }

    fn eat(&mut self, expected: char) -> bool {
        let found = self.chars.peek() == Some(&expected);
        if found {
            self.bump();
        }
        found
    }

    fn next(&mut self) -> Result<Token, Diagnostic> {
        self.skip_blanks();
        let pos = self.pos;
        let Some(c) = self.bump() else {
            return Ok(Token { tok: Tok::Eof, pos });
        };
        let tok = match c {
            '\n' => Tok::Newline,
            '#' => {
                self.take_while(|c| c != '\n');
                return self.next();
            }
            ';' => Tok::Comment(self.take_while(|c| c != '\n').trim().into()),
            '"' => {
                let text = self.take_while(|c| c != '"' && c != '\n');
                if !self.eat('"') {
                    return Err(Diagnostic::error(pos, "string is not closed"));
                }
                Tok::Str(text)
            }
            '\'' => {
                let key = self.bump().filter(|&c| c != '\n');
                match key {
                    Some(key) if self.eat('\'') => Tok::Key(key),
                    _ => return Err(Diagnostic::error(pos, "expected a key like '+'")),
                }
            }
            c if c.is_ascii_digit() => self.number(c, pos)?,
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut name = c.to_string();
                name.push_str(&self.take_while(|c| c.is_ascii_alphanumeric() || c == '_'));
                if name == "asm" {
                    self.asm(pos)?
                } else {
                    Tok::Ident(name)
                }
            }
            c => self.symbol(c, pos)?,
        };
        Ok(Token { tok, pos })
    }

    fn number(&mut self, first: char, pos: Pos) -> Result<Tok, Diagnostic> {
        let mut text = first.to_string();
        text.push_str(&self.take_while(|c| c.is_ascii_alphanumeric()));
        let parsed = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            Some(hex) => u8::from_str_radix(hex, 16),
            None => text.parse(),
        };
        let value =
            parsed.map_err(|_| Diagnostic::error(pos, format!("invalid number '{text}'")))?;
        Ok(Tok::Number(Literal { value, text }))
    }

    fn skip_blanks(&mut self) {
        while self
            .chars
            .peek()
            .is_some_and(|&c| c != '\n' && c.is_whitespace())
        {
            self.bump();
        }
    }

    fn symbol(&mut self, first: char, pos: Pos) -> Result<Tok, Diagnostic> {
        let second = self.chars.peek().copied();
        let mut pair = first.to_string();
        pair.extend(second);
        if let Some(&sym) = SYMBOLS.iter().find(|&&s| s.len() == 2 && s == pair) {
            self.bump();
            return Ok(Tok::Sym(sym));
        }
        SYMBOLS
            .iter()
            .find(|s| s.len() == 1 && s.starts_with(first))
            .map(|&s| Tok::Sym(s))
            .ok_or_else(|| Diagnostic::error(pos, format!("unexpected character '{first}'")))
    }

    fn take_while(&mut self, keep: impl Fn(char) -> bool) -> String {
        let mut text = String::new();
        while let Some(&c) = self.chars.peek() {
            if !keep(c) {
                break;
            }
            text.push(c);
            self.bump();
        }
        text
    }
}

pub(crate) fn lex(source: &str) -> Result<Vec<Token>, Diagnostic> {
    let mut lexer = Lexer {
        chars: source.chars().peekable(),
        pos: Pos { line: 1, col: 1 },
    };
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next()?;
        let done = token.tok == Tok::Eof;
        tokens.push(token);
        if done {
            return Ok(tokens);
        }
    }
}
