use std::path::Path;
use std::sync::LazyLock;

use ld60_dsl::{Flavor, Selection, Target, compile, compile_pair};
use ld60_levels::{Catalog, Variant};

const LEVELS: &str = concat!(env!("LD60_ASSETS"), "/levels");
static CATALOG: LazyLock<Catalog> = LazyLock::new(|| Catalog::read(Path::new(LEVELS)).unwrap());

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{LEVELS}/{path}")).unwrap()
}

#[test]
fn generated_firmware_is_current() {
    let mut stale = Vec::new();
    for source in CATALOG.levels() {
        let level = source.load().unwrap();
        let Some(firmware) = &level.firmware else {
            continue;
        };
        let text = read(&firmware.source);
        let target = Target::new(level.chip, level.board());
        let pair = match compile_pair(&text, &target, &firmware.switches) {
            Ok(pair) => pair,
            Err(errors) => {
                let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
                panic!("{}: {}", firmware.source, errors.join("\n"));
            }
        };
        let generated = Variant {
            level: level.clone(),
            buggy: pair.buggy.asm.clone(),
            fixed: pair.fixed.asm.clone(),
        };
        if let Err(error) = generated.verify() {
            panic!("{}: {error}", source.path);
        }
        for (name, generated, committed) in [
            ("buggy.asm", &pair.buggy.asm, &source.buggy),
            ("fixed.asm", &pair.fixed.asm, &source.fixed),
        ] {
            if generated != committed {
                stale.push(format!("{}/{name}\n{generated}", source.path));
            }
        }
    }
    assert!(
        stale.is_empty(),
        "stale, run 'cli gen':\n{}",
        stale.join("\n")
    );
}

#[test]
fn every_level_is_generated() {
    let generated = CATALOG
        .levels()
        .iter()
        .filter(|s| s.load().unwrap().firmware.is_some())
        .count();
    assert_eq!(generated, CATALOG.levels().len());
}

#[test]
fn firmware_1_0_built_for_rev_a_fixes_the_clear_key_level() {
    let source = CATALOG.find("tally2/01-clear-key").unwrap();
    let level = source.load().unwrap();
    let selection = Selection {
        flavor: Flavor::Fixed,
        switches: &[],
    };
    let target = Target::new(level.chip, level.board());
    let fixed = compile(&read("tally2/firmware-1.0.fw"), &target, &selection).unwrap();
    let variant = Variant {
        level,
        buggy: source.buggy.clone(),
        fixed: fixed.asm,
    };
    variant.verify().unwrap();
}
