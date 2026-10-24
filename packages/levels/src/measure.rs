use ld60_devices::{Device, MAX_STEPS, Rom};

use crate::{Level, LevelError, TEST_SEEDS, VerifyError, assemble, burn, device, initial_rom};

/// How big a repair is and how fast the repaired device works.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Measure {
    /// ROM words that differ from the firmware the device came in with.
    pub words: u32,
    /// CPU steps the device takes for all customer tests.
    pub cycles: u32,
}

impl Measure {
    /// Measures `after` against the ROM the device came in with.
    pub fn new(level: &Level, buggy: &str, after: &Rom) -> Result<Self, LevelError> {
        let before = initial_rom(level, buggy)?;
        Ok(Self {
            words: Self::changed_words(&before, after),
            cycles: Self::cycles(level, after)?,
        })
    }

    /// Measures the repair that `fixed` makes.
    pub fn par(level: &Level, buggy: &str, fixed: &str) -> Result<Self, VerifyError> {
        let before = initial_rom(level, buggy).map_err(VerifyError::Buggy)?;
        let image = assemble(level, fixed).map_err(VerifyError::Fixed)?.image;
        let after = burn(level, &before, image).map_err(VerifyError::Burn)?;
        Self::new(level, buggy, &after).map_err(VerifyError::Run)
    }

    /// Whether this repair changes no more words than `par`.
    pub fn neat(self, par: Self) -> bool {
        self.words <= par.words
    }

    /// Whether this repair runs no slower than `par`.
    pub fn quick(self, par: Self) -> bool {
        self.cycles <= par.cycles
    }

    fn changed_words(before: &Rom, after: &Rom) -> u32 {
        let changed = before
            .image()
            .words()
            .iter()
            .zip(after.image().words())
            .filter(|(a, b)| a != b)
            .count();
        u32::try_from(changed).unwrap_or(u32::MAX)
    }

    /// A test that never halts counts as [`MAX_STEPS`] for each step.
    fn cycles(level: &Level, rom: &Rom) -> Result<u32, LevelError> {
        let mut device = device(level, rom.clone())?;
        let mut total = 0_u32;
        for case in &level.tests {
            device.power_on(TEST_SEEDS[0]);
            let mut steps = device.run_until_halt(MAX_STEPS).unwrap_or(MAX_STEPS);
            for key in case.keys.chars().filter(|c| !c.is_whitespace()) {
                if device.press_key(key).is_ok() {
                    steps += device.run_until_halt(MAX_STEPS).unwrap_or(MAX_STEPS);
                }
            }
            total = total.saturating_add(steps);
        }
        Ok(total)
    }
}
