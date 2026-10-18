use std::io::Write;

use ld60_levels::{TEST_SEEDS, Variant, assemble, burn, initial_rom};

/// Returns whether all tests passed.
pub fn run(out: &mut impl Write, variant: &Variant, file: Option<&str>) -> crate::Result<bool> {
    let level = &variant.level;
    writeln!(out, "Board {}", level.board().name())?;
    let initial = initial_rom(level, &variant.buggy)?;
    let rom = match file {
        None => initial,
        Some(path) => {
            let image = assemble(level, &std::fs::read_to_string(path)?)?.image;
            burn(level, &initial, image)?
        }
    };
    let report = ld60_levels::run(level, &rom, TEST_SEEDS)?;
    write!(out, "{report}")?;
    Ok(report.passed())
}
