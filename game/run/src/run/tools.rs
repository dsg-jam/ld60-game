use serde::{Deserialize, Serialize};

/// A program for the shop computer that helps find bugs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputerTool {
    /// Runs the complaint of the job against the firmware.
    TestBench,
    /// Runs the firmware one instruction at a time.
    Debugger,
    /// Shows the last lines the firmware ran.
    Trace,
    /// Stops the firmware at marked lines.
    Breakpoints,
}

impl ComputerTool {
    pub(crate) const ALL: [Self; 4] = [
        Self::TestBench,
        Self::Debugger,
        Self::Trace,
        Self::Breakpoints,
    ];

    /// The tool this one only works with.
    pub(crate) const fn needs(self) -> Option<Self> {
        match self {
            Self::Breakpoints => Some(Self::Debugger),
            Self::TestBench | Self::Debugger | Self::Trace => None,
        }
    }
}
