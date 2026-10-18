mod effects;
mod firmware;
mod line;
mod ram_map;
mod rng;

use ld60_devices::Fault;

use self::firmware::Firmware;
use self::ram_map::RamMap;
use self::rng::Rng;
use crate::{
    Level, LevelError, LevelSource, TEST_SEEDS, VerifyError, assemble, burn, initial_rom, run,
};

const TRIES: u64 = 8;

/// A level with its firmware rewritten into an equivalent layout.
///
/// The rewrite renames free RAM cells, swaps independent instructions and
/// moves routines. Faults follow the renamed cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub level: Level,
    pub buggy: String,
    pub fixed: String,
}

impl Variant {
    /// Returns a rewrite for `seed` that still starts broken and can be fixed.
    ///
    /// Falls back to the original level if no rewrite passes.
    pub fn new(source: &LevelSource, seed: u64) -> Result<Self, LevelError> {
        let original = Self::original(source)?;
        let rom_damage = original
            .level
            .faults
            .iter()
            .any(|f| matches!(f, Fault::FadedRomBits { .. }));
        // ponytail: faded ROM bits sit at fixed addresses, so these levels keep their layout.
        if rom_damage {
            return Ok(original);
        }
        let variant = (0..TRIES)
            .map(|n| original.rewrite(seed.wrapping_add(n)))
            .find(|v| v.verify().is_ok());
        Ok(variant.unwrap_or(original))
    }

    pub fn original(source: &LevelSource) -> Result<Self, LevelError> {
        Ok(Self {
            level: source.load()?,
            buggy: source.buggy.into(),
            fixed: source.fixed.into(),
        })
    }

    fn rewrite(&self, seed: u64) -> Self {
        let mut buggy = Firmware::new(&self.buggy);
        let mut fixed = Firmware::new(&self.fixed);
        let pinned = self
            .level
            .device
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
        for fault in &mut level.faults {
            if let Fault::StuckRamBit(bit) = fault {
                bit.cell = map.get(bit.cell);
            }
        }
        Self {
            level,
            buggy: buggy.to_string(),
            fixed: fixed.to_string(),
        }
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
