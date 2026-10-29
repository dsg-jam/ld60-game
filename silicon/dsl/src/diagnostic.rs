use std::fmt;

/// A 1-based position in the source. Line 0 means no position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Pos {
    pub(crate) line: usize,
    pub(crate) col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn error(pos: Pos, message: impl Into<String>) -> Self {
        Self::new(pos, Severity::Error, message)
    }

    pub(crate) fn new(pos: Pos, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            line: pos.line,
            col: pos.col,
            severity,
            message: message.into(),
        }
    }

    pub(crate) fn warning(pos: Pos, message: impl Into<String>) -> Self {
        Self::new(pos, Severity::Warning, message)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "{}:{}: ", self.line, self.col)?;
        }
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{severity}: {}", self.message)
    }
}

impl std::error::Error for Diagnostic {}
