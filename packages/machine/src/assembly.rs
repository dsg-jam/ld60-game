use std::collections::BTreeMap;
use std::fmt;

use crate::Image;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AsmError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for AsmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for AsmError {}

/// Maps emitted words back to a 1-based source line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLine {
    pub line: usize,
    pub addr: u16,
    pub len: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assembly {
    pub image: Image,
    pub lines: Vec<SourceLine>,
    pub labels: BTreeMap<String, u16>,
}

impl Assembly {
    pub fn line_at(&self, addr: u16) -> Option<usize> {
        self.lines
            .iter()
            .find(|l| (l.addr..l.addr + l.len).contains(&addr))
            .map(|l| l.line)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisasmLine {
    pub addr: u16,
    pub words: Vec<u16>,
    pub text: String,
}
