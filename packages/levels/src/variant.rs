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
};

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
        original.on_board(boards[pick as usize], seed)
    }

    /// Like [`Variant::new`], but on the given board.
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
            buggy: source.buggy.into(),
            fixed: source.fixed.into(),
        })
    }

    fn on_board(self, board: BoardRev, seed: u64) -> Result<Self, LevelError> {
        if !self.level.allowed_boards().contains(&board) {
            return Err(LevelError::BoardNotAllowed(board));
        }
        // ponytail: faded ROM bits sit at fixed addresses, so these levels keep their layout.
        if self.level.has_rom_damage() {
            return Ok(self);
        }
        let variant = (0..TRIES)
            .filter_map(|n| self.rewrite(board, seed.wrapping_add(n)))
            .find(|v| v.verify().is_ok());
        Ok(variant.unwrap_or(self))
    }

    /// Returns `None` if the firmware cannot move to `board`.
    fn rewrite(&self, board: BoardRev, seed: u64) -> Option<Self> {
        let (from, to) = (self.level.board(), self.level.device.board(board));
        let mut buggy = Firmware::new(&self.buggy);
        let mut fixed = Firmware::new(&self.fixed);
        let rotation = buggy.rewire(from, to)?;
        fixed.rewire(from, to)?;
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
        let mut level = self.level.clone();
        level.board = board;
        if let Some(number) = level.serial.strip_prefix(&from.name()) {
            level.serial = format!("{}{number}", to.name());
        }
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

    /// Checks that the buggy firmware fails a test and the fixed one passes all.
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
        Ok(())
    }
}
