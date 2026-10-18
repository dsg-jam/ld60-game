use std::fmt;

use ld60_devices::{Board, ChipSpec};

use crate::backend::{Backend, CalcoBackend};

/// The board and chip a firmware is compiled for.
pub struct Target {
    pub(crate) backend: Box<dyn Backend>,
    pub(crate) board: &'static Board,
    pub(crate) chip: ChipSpec,
}

impl Target {
    pub fn new(chip: ChipSpec, board: &'static Board) -> Self {
        let backend = match chip {
            ChipSpec::Calco(info) => Box::new(CalcoBackend::new(info.revision)),
        };
        Self {
            backend,
            board,
            chip,
        }
    }

    /// The name used after `for` in a firmware declaration.
    pub(crate) fn device_name(&self) -> String {
        format!("{:?}", self.board.kind).to_ascii_lowercase()
    }

    /// The code the firmware reads from the key port for `key`.
    pub(crate) fn key_code(&self, key: char) -> Option<u8> {
        self.board.key_code(key)
    }
}

impl fmt::Debug for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Target")
            .field("board", &self.board.name())
            .field("chip", &self.chip)
            .finish_non_exhaustive()
    }
}
