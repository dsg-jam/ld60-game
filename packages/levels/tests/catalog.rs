use std::collections::HashSet;

use ld60_levels::{CATALOG, Variant};

#[test]
fn every_level_is_solvable_and_starts_broken() {
    let mut failures = Vec::new();
    let mut ids = HashSet::new();
    for source in CATALOG {
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
    let chips: Vec<&str> = CATALOG[..2]
        .iter()
        .map(|source| source.load().unwrap().chip.isa().name())
        .collect();
    assert_eq!(chips, ["CalCo Rev A", "CalCo Rev B"]);
}

#[test]
fn variants_verify_over_many_seeds() {
    for source in CATALOG {
        for seed in 0..64 {
            let variant = Variant::new(source, seed).unwrap();
            if let Err(error) = variant.verify() {
                panic!("{} seed {seed}: {error}\n{}", source.path, variant.buggy);
            }
        }
    }
}

#[test]
fn every_level_has_working_variants() {
    for source in CATALOG {
        let original = Variant::original(source).unwrap();
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
