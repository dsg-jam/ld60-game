mod differential;
mod effects;
mod firmware;
mod line;
mod ram_map;
mod rng;

use ld60_devices::{BoardRev, Fault};

use self::firmware::Firmware;
use self::ram_map::RamMap;
use self::rng::Rng;
use crate::{
    Level, LevelError, LevelSource, TEST_SEEDS, VerifyError, assemble, burn, initial_rom, run,
    run_line_tests,
};

/// The seed the game uses for its jobs.
pub const GAME_SEED: u64 = 0x1D60;

const TRIES: u64 = 8;

/// A level with its firmware rewritten into an equivalent layout.
///
/// The rewrite moves the job to another board revision, renames free RAM
/// cells, swaps independent instructions and moves routines. Faults follow
/// the renamed cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub level: Level,
    pub buggy: String,
    pub fixed: String,
}

impl Variant {
    /// Returns a rewrite for `seed` that still starts broken and can be fixed.
    ///
    /// The seed also picks one of the allowed boards. Falls back to the
    /// original level if no rewrite passes.
    pub fn new(source: &LevelSource, seed: u64) -> Result<Self, LevelError> {
        let original = Self::original(source)?;
        let boards = original.level.allowed_boards();
        let pick = Rng::new(seed, &original.level.id).next_u64() % boards.len() as u64;
        match original.clone().on_board(boards[pick as usize], seed) {
            Err(LevelError::NoRewrite(_)) => Ok(original),
            result => result,
        }
    }

    /// Like [`Variant::new`], but on the given board and without a fallback.
    pub fn with_board(
        source: &LevelSource,
        seed: u64,
        board: BoardRev,
    ) -> Result<Self, LevelError> {
        Self::original(source)?.on_board(board, seed)
    }

    pub fn original(source: &LevelSource) -> Result<Self, LevelError> {
        Ok(Self {
            level: source.load()?,
            buggy: source.buggy.clone(),
            fixed: source.fixed.clone(),
        })
    }

    fn on_board(self, board: BoardRev, seed: u64) -> Result<Self, LevelError> {
        if !self.level.allowed_boards().contains(&board) {
            return Err(LevelError::BoardNotAllowed(board));
        }
        if board == self.level.board && self.level.keeps_layout() {
            return Ok(self);
        }
        (0..TRIES)
            .filter_map(|n| self.rewrite(board, seed.wrapping_add(n)))
            .find(|v| v.verify().is_ok() && v.behaves_like(&self))
            .ok_or(LevelError::NoRewrite(board))
    }

    /// Returns `None` if the firmware cannot move to `board`.
    fn rewrite(&self, board: BoardRev, seed: u64) -> Option<Self> {
        let (from, to) = (self.level.board(), self.level.device.board(board));
        let mut buggy = Firmware::new(&self.buggy);
        let mut fixed = Firmware::new(&self.fixed);
        let rotation = buggy.rewire(from, to)?;
        fixed.rewire(from, to)?;
        let map = if self.level.keeps_layout() {
            RamMap::rotate(0)
        } else {
            let pinned = to
                .display_cells()
                .chain(buggy.indexed_cells())
                .chain(fixed.indexed_cells());
            let map = RamMap::shuffle(pinned, &mut Rng::new(seed, "ram"));
            for firmware in [&mut buggy, &mut fixed] {
                firmware.remap_ram(&map);
                firmware.swap_independent(seed);
                firmware.shuffle_key_checks(seed);
                firmware.reorder_routines(seed);
            }
            map
        };
        let mut level = self.level.clone();
        level.board = board;
        for fault in &mut level.faults {
            if let Fault::StuckRamBit(bit) = fault {
                bit.cell = map.get(rotation.get(bit.cell));
            }
        }
        Some(Self {
            level,
            buggy: buggy.to_string(),
            fixed: fixed.to_string(),
        })
    }

    /// Checks that the buggy firmware fails a test and the fixed one passes all,
    /// including the end of line tests of the device.
    pub fn verify(&self) -> Result<(), VerifyError> {
        let level = &self.level;
        let initial = initial_rom(level, &self.buggy).map_err(VerifyError::Buggy)?;
        let report = run(level, &initial, TEST_SEEDS).map_err(VerifyError::Run)?;
        if report.passed() {
            return Err(VerifyError::BuggyPasses(report));
        }
        let fixed = assemble(level, &self.fixed)
            .map_err(VerifyError::Fixed)?
            .image;
        let rom = burn(level, &initial, fixed).map_err(VerifyError::Burn)?;
        let report = run(level, &rom, TEST_SEEDS).map_err(VerifyError::Run)?;
        if !report.passed() {
            return Err(VerifyError::FixedFails(report));
        }
        let report = run_line_tests(level, &rom, TEST_SEEDS).map_err(VerifyError::Run)?;
        if !report.passed() {
            return Err(VerifyError::FixedFailsLineTests(report));
        }
        Ok(())
    }
}
