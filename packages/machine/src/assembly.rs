use std::collections::BTreeMap;

use crate::Image;

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
    /// Bytes of all instructions and data.
    pub fn byte_len(&self) -> u16 {
        self.lines.iter().map(|line| line.len).sum()
    }

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
