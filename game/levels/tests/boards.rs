use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use ld60_devices::{BoardRev, Fault};
use ld60_levels::{Catalog, GAME_SEED, LevelError, LevelSource, Variant};

const LEVELS: &str = concat!(env!("LD60_ASSETS"), "/levels");
static CATALOG: LazyLock<Catalog> = LazyLock::new(|| Catalog::read(Path::new(LEVELS)).unwrap());

const SEEDS: std::ops::Range<u64> = 0..256;
/// Levels whose buggy firmware checks a wrong key code or misses a check.
const WRONG_KEY_CODE: &[&str] = &["tally2/t4-c-key-counts", "tally2/t5-no-clear-key"];
/// Tutorial firmware that only checks for the C key. Any other key counts.
const ONLY_CLEAR_KEY: &[&str] = &[
    "tally2/t4-c-key-counts",
    "tally2/t5-no-clear-key",
    "tally2/t6-squiggle-after-nine",
];

fn find(path: &str) -> &'static LevelSource {
    CATALOG.find(path).unwrap()
}

/// The cell named at the start of the header comment that contains `text`.
fn header_cell(firmware: &str, text: &str) -> u8 {
    let line = firmware
        .lines()
        .find(|line| line.starts_with(';') && line.contains(text))
        .unwrap();
    let cell = line.trim_start_matches(';').trim_start();
    u8::from_str_radix(&cell[1..2], 16).unwrap()
}

fn compares(firmware: &str) -> HashSet<u8> {
    firmware
        .lines()
        .filter_map(|line| {
            let value = line.trim().strip_prefix("CPI")?.split_whitespace().next()?;
            match value.strip_prefix("0x") {
                Some(hex) => u8::from_str_radix(hex, 16).ok(),
                None => value.parse().ok(),
            }
        })
        .collect()
}

#[test]
fn every_level_verifies_on_every_allowed_board() {
    let mut failures = Vec::new();
    for source in CATALOG.levels() {
        let level = source.load().unwrap();
        for board in level.allowed_boards() {
            for seed in SEEDS.chain([GAME_SEED]) {
                let result = Variant::with_board(source, seed, board)
                    .map_err(|e| e.to_string())
                    .and_then(|v| match v.verify() {
                        Ok(()) if v.level.board == board => Ok(()),
                        Ok(()) => Err(format!("on board {:?}", v.level.board)),
                        Err(error) => Err(error.to_string()),
                    });
                if let Err(error) = result {
                    failures.push(format!(
                        "{} board {board:?} seed {seed}: {error}",
                        source.path
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn board_choice_is_deterministic_and_uses_every_board() {
    for source in CATALOG.levels() {
        let mut seen = HashSet::new();
        for seed in 0..32 {
            let variant = Variant::new(source, seed).unwrap();
            assert_eq!(variant, Variant::new(source, seed).unwrap());
            seen.insert(variant.level.board);
        }
        let allowed: HashSet<BoardRev> = source
            .load()
            .unwrap()
            .allowed_boards()
            .into_iter()
            .collect();
        assert_eq!(seen, allowed, "{}", source.path);
    }
}

#[test]
fn key_checks_use_the_board_codes() {
    for source in CATALOG.levels() {
        let level = source.load().unwrap();
        // Firmware this old does not read keys yet.
        if level.before_line_tests {
            continue;
        }
        for board in level.allowed_boards() {
            let variant = Variant::with_board(source, 3, board).unwrap();
            let board = variant.level.board();
            let mut firmwares = vec![&variant.fixed];
            if !WRONG_KEY_CODE.contains(&source.path.as_str()) {
                firmwares.push(&variant.buggy);
            }
            for firmware in firmwares {
                let checks = compares(firmware);
                let keys: &[char] = if ONLY_CLEAR_KEY.contains(&source.path.as_str()) {
                    &['C']
                } else {
                    &['+', '*', 'C']
                };
                for &key in keys {
                    let Some(code) = board.key_code(key) else {
                        continue;
                    };
                    assert!(checks.contains(&code), "{} {key}\n{firmware}", source.path);
                }
            }
        }
    }
}

#[test]
fn display_comment_names_the_board_cells() {
    for source in CATALOG.levels() {
        for board in source.load().unwrap().allowed_boards() {
            let variant = Variant::with_board(source, 5, board).unwrap();
            let first = *variant.level.board().display_cells().start();
            for firmware in [&variant.buggy, &variant.fixed] {
                let text = if source.path.starts_with("tally") {
                    "count"
                } else {
                    "display"
                };
                assert_eq!(
                    usize::from(header_cell(firmware, text)),
                    first,
                    "{}",
                    source.path
                );
            }
        }
    }
}

#[test]
fn stuck_bit_follows_the_cell() {
    let source = find("summa6/04-coffee");
    for board in BoardRev::ALL {
        for seed in 0..16 {
            let variant = Variant::with_board(source, seed, board).unwrap();
            let [Fault::StuckRamBit(bit)] = variant.level.faults[..] else {
                panic!("expected one stuck bit");
            };
            assert_eq!(bit.cell, header_cell(&variant.buggy, "pending operation"));
        }
    }
}

#[test]
fn serial_shows_the_board() {
    let source = find("summa6/02-last-digit");
    let variant = Variant::with_board(source, 1, BoardRev::C).unwrap();
    assert_eq!(variant.level.serial(), "6C-03702");
}

#[test]
fn first_jobs_keep_board_a_and_their_layout() {
    for path in ["tally2/01-clear-key", "summa6/01-strange-symbols"] {
        let source = find(path);
        let original = Variant::original(source).unwrap();
        assert_eq!(original.level.board, BoardRev::A);
        for seed in SEEDS.chain([GAME_SEED]) {
            assert_eq!(Variant::new(source, seed).unwrap(), original, "{path}");
        }
    }
}

#[test]
fn levels_can_restrict_boards() {
    let source = find("summa6/02-last-digit");
    let level = source
        .level
        .replace("device: Summa6,", "device: Summa6,\n    boards: [A],");
    let source = LevelSource {
        level,
        ..source.clone()
    };
    assert_eq!(
        Variant::with_board(&source, 0, BoardRev::B),
        Err(LevelError::BoardNotAllowed(BoardRev::B))
    );
    for seed in 0..16 {
        let variant = Variant::new(&source, seed).unwrap();
        assert_eq!(variant.level.board, BoardRev::A);
    }
}
