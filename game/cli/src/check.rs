use std::io::Write;

use ld60_levels::{TEST_SEEDS, Variant, assemble, burn, initial_rom, run_line_tests};

/// Runs the customer's tests, then the end of line tests. Returns whether
/// all tests passed.
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
    writeln!(out, "Customer tests")?;
    write!(out, "{report}")?;
    let line_tests = run_line_tests(level, &rom, TEST_SEEDS)?;
    writeln!(out, "End of line tests")?;
    write!(out, "{line_tests}")?;
    Ok(report.passed() && line_tests.passed())
}
