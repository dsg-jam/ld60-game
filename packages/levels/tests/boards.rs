use std::collections::HashSet;

use ld60_devices::{BoardRev, Fault};
use ld60_levels::{CATALOG, LevelError, LevelSource, Variant};

const SEEDS: std::ops::Range<u64> = 0..256;

fn find(path: &str) -> &'static LevelSource {
    CATALOG.iter().find(|s| s.path == path).unwrap()
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
            let value = line.trim().strip_prefix("CPI")?.trim();
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
    for source in CATALOG {
        let level = source.load().unwrap();
        for board in level.allowed_boards() {
            for seed in SEEDS {
                let variant = Variant::with_board(source, seed, board).unwrap();
                if variant.level.board != board {
                    failures.push(format!(
                        "{} board {board:?} seed {seed}: fell back",
                        source.path
                    ));
                } else if let Err(error) = variant.verify() {
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
    for source in CATALOG {
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
    for source in CATALOG {
        for board in BoardRev::ALL {
            let variant = Variant::with_board(source, 3, board).unwrap();
            let board = variant.level.board();
            for firmware in [&variant.buggy, &variant.fixed] {
                let checks = compares(firmware);
                for key in ['+', 'C'] {
                    let code = board.key_code(key).unwrap();
                    assert!(checks.contains(&code), "{} {key}\n{firmware}", source.path);
                }
            }
        }
    }
}

#[test]
fn display_comment_names_the_board_cells() {
    for source in CATALOG {
        for board in BoardRev::ALL {
            let variant = Variant::with_board(source, 5, board).unwrap();
            let first = *variant.level.board().display_cells().start();
            for firmware in [&variant.buggy, &variant.fixed] {
                let text = if source.path.starts_with("tally2") {
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
    assert_eq!(variant.level.serial, "6C-03702");
}

#[test]
fn levels_can_restrict_boards() {
    let source = find("summa6/02-last-digit");
    let level = source
        .level
        .replace("device: Summa6,", "device: Summa6,\n    boards: [A],");
    let source = LevelSource {
        level: Box::leak(level.into_boxed_str()),
        ..*source
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
