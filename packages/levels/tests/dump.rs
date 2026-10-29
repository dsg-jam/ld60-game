use std::path::Path;
use std::sync::LazyLock;

use ld60_levels::{Catalog, assemble, dumped_source, initial_rom};

const LEVELS: &str = concat!(env!("LD60_ASSETS"), "/levels");
static CATALOG: LazyLock<Catalog> = LazyLock::new(|| Catalog::read(Path::new(LEVELS)).unwrap());

#[test]
fn reading_a_faded_rom_shows_the_faded_line_as_it_is_on_the_chip() {
    let source = CATALOG.find("lock4/04-sunburn").unwrap();
    let level = source.load().unwrap();
    let rom = initial_rom(&level, &source.buggy).unwrap();
    let dumped = dumped_source(&level, &source.buggy, rom.image()).unwrap();
    let changed: Vec<_> = source
        .buggy
        .lines()
        .zip(dumped.lines())
        .filter(|(buggy, dumped)| buggy != dumped)
        .collect();
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert_eq!(
        assemble(&level, &dumped).unwrap().image,
        rom.image().clone()
    );
}

#[test]
fn reading_an_undamaged_rom_shows_the_firmware_as_written() {
    let source = CATALOG.find("tally2/01-clear-key").unwrap();
    let level = source.load().unwrap();
    let rom = initial_rom(&level, &source.buggy).unwrap();
    assert_eq!(
        dumped_source(&level, &source.buggy, rom.image()).unwrap(),
        source.buggy
    );
}
