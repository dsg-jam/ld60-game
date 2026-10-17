use std::io::Write;

use levels::{TEST_SEEDS, assemble, burn, initial_rom};

use crate::find_level;

/// Returns whether all tests passed.
pub fn run(out: &mut impl Write, level: &str, file: Option<&str>) -> crate::Result<bool> {
    let source = find_level(level)?;
    let level = source.load()?;
    let initial = initial_rom(&level, source.buggy)?;
    let rom = match file {
        None => initial,
        Some(path) => {
            let image = assemble(&level, &std::fs::read_to_string(path)?)?.image;
            burn(&level, &initial, image)?
        }
    };
    let report = levels::run(&level, &rom, TEST_SEEDS)?;
    write!(out, "{report}")?;
    Ok(report.passed())
}
