use std::collections::HashSet;

use levels::{CATALOG, Level, LevelSource, TEST_SEEDS, assemble, burn, initial_rom, run};

fn verify(source: &LevelSource) -> Result<Level, String> {
    let level = source.load().map_err(|e| e.to_string())?;
    let initial = initial_rom(&level, source.buggy).map_err(|e| format!("buggy.asm: {e}"))?;
    let report = run(&level, &initial, TEST_SEEDS).map_err(|e| e.to_string())?;
    if report.passed() {
        return Err(format!("buggy.asm passes all tests\n{report}"));
    }

    let fixed = assemble(&level, source.fixed)
        .map_err(|e| format!("fixed.asm: {e}"))?
        .image;
    let rom = burn(&level, &initial, fixed)
        .map_err(|e| format!("fixed.asm cannot be burned over the chip: {e}"))?;
    let report = run(&level, &rom, TEST_SEEDS).map_err(|e| e.to_string())?;
    if !report.passed() {
        return Err(format!("fixed.asm fails\n{report}"));
    }
    Ok(level)
}

#[test]
fn every_level_is_solvable_and_starts_broken() {
    let mut failures = Vec::new();
    let mut ids = HashSet::new();
    for source in CATALOG {
        match verify(source) {
            Ok(level) if !ids.insert(level.id.clone()) => {
                failures.push(format!("{}: duplicate id {}", source.path, level.id));
            }
            Ok(_) => {}
            Err(message) => failures.push(format!("{}: {message}", source.path)),
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
