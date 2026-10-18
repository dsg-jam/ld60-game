use std::collections::HashSet;
use std::sync::LazyLock;

use ld60_levels::{Catalog, TEST_SEEDS, Variant, initial_rom, run_line_tests};

static CATALOG: LazyLock<Catalog> = LazyLock::new(|| Catalog::on_disk().unwrap());

/// Levels whose bug only the customer's tests find.
const MISSED_BY_LINE_TESTS: &[&str] = &["summa6-05", "summa6-06"];

#[test]
fn line_tests_catch_the_bug_unless_listed() {
    let mut failures = Vec::new();
    for source in CATALOG.levels() {
        let original = Variant::original(source).unwrap();
        let level = &original.level;
        let rom = initial_rom(level, &original.buggy).unwrap();
        let report = run_line_tests(level, &rom, TEST_SEEDS).unwrap();
        let listed = MISSED_BY_LINE_TESTS.contains(&level.id.as_str());
        if report.passed() != listed {
            failures.push(format!("{}: listed {listed}\n{report}", level.id));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn every_level_is_solvable_and_starts_broken() {
    let mut failures = Vec::new();
    let mut ids = HashSet::new();
    for source in CATALOG.levels() {
        match Variant::original(source) {
            Ok(original) => match original.verify() {
                Ok(()) if !ids.insert(original.level.id.clone()) => {
                    failures.push(format!(
                        "{}: duplicate id {}",
                        source.path, original.level.id
                    ));
                }
                Ok(()) => {}
                Err(error) => failures.push(format!("{}: {error}", source.path)),
            },
            Err(error) => failures.push(format!("{}: {error}", source.path)),
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

#[test]
fn tutorial_starts_on_the_smallest_chip() {
    let chips: Vec<&str> = CATALOG.levels()[..2]
        .iter()
        .map(|source| source.load().unwrap().chip.isa().name())
        .collect();
    assert_eq!(chips, ["CalCo Rev A", "CalCo Rev B"]);
}

/// `Variant::new` falls back to the original when no rewrite verifies, so
/// most seeds must give a different firmware.
#[test]
fn most_seeds_give_a_rewrite() {
    const SEEDS: u64 = 64;
    for source in CATALOG.levels() {
        let original = Variant::original(source).unwrap();
        if original.level.keeps_layout() {
            continue;
        }
        let rewrites = (0..SEEDS)
            .filter(|&seed| Variant::new(source, seed).unwrap().buggy != original.buggy)
            .count();
        assert!(
            rewrites as u64 * 4 >= SEEDS * 3,
            "{}: {rewrites} of {SEEDS} seeds give a rewrite",
            source.path
        );
    }
}

#[test]
fn every_level_has_working_variants() {
    for source in CATALOG.levels() {
        let original = Variant::original(source).unwrap();
        if original.level.keeps_layout() && original.level.allowed_boards().len() == 1 {
            continue;
        }
        let variants: HashSet<String> = (0..16)
            .map(|seed| {
                let variant = Variant::new(source, seed).unwrap();
                if let Err(error) = variant.verify() {
                    panic!("{} seed {seed}: {error}\n{}", source.path, variant.buggy);
                }
                variant.buggy
            })
            .filter(|buggy| *buggy != original.buggy)
            .collect();
        assert!(
            variants.len() > 8,
            "{}: {} variants",
            source.path,
            variants.len()
        );
    }
}
