mod common;

use common::{LOCK, SUMMA, TALLY_0_9, TALLY_1_0, build, tally};
use ld60_calco::{ChipInfo, DateCode, Revision};
use ld60_devices::{Annunciator, BoardRev, ChipSpec, DeviceKind};
use ld60_dsl::{Flavor, Selection, Target, compile};
use ld60_machine::Isa;

const SUMMA_SWITCHES: &[&str] = &[
    "1.1",
    "coffee",
    "deep-call",
    "ds-errata",
    "negative",
    "nine-key",
    "no-clear",
    "short-add",
];

#[test]
fn output_is_deterministic() {
    for &switch in SUMMA_SWITCHES {
        for flavor in [Flavor::Buggy, Flavor::Fixed] {
            let first = build(SUMMA, Revision::B, DeviceKind::Summa6, flavor, &[switch]);
            let second = build(SUMMA, Revision::B, DeviceKind::Summa6, flavor, &[switch]);
            assert_eq!(first, second, "{switch}");
        }
    }
}

#[test]
fn every_switch_assembles_on_every_supported_revision() {
    for revision in [Revision::B, Revision::C] {
        for &switch in SUMMA_SWITCHES {
            for flavor in [Flavor::Buggy, Flavor::Fixed] {
                let output = build(SUMMA, revision, DeviceKind::Summa6, flavor, &[switch]).unwrap();
                assert!(revision.isa().assemble(&output.asm).is_ok(), "{switch}");
            }
        }
    }
    for (source, device, revisions) in [
        (TALLY_0_9, DeviceKind::Tally2, &Revision::ALL[..]),
        (TALLY_1_0, DeviceKind::Tally2, &Revision::ALL[..]),
        (LOCK, DeviceKind::Lock4, &[Revision::C][..]),
    ] {
        for &revision in revisions {
            let output = build(source, revision, device, Flavor::Fixed, &[]).unwrap();
            assert!(
                revision.isa().assemble(&output.asm).is_ok(),
                "Rev {revision}"
            );
        }
    }
}

#[test]
fn layout_matches_hand_written_files() {
    let source = "\
firmware \"Layout\" for tally2
cells tally[2] at M1    ; count
cell spare at M3
; Notes go into the header.

entry reset {
    ; Clear the count.
    a = 0
    tally[1] = a        ; tens
    goto countdown
}

block countdown {
    asm {
        NOP
        here: NOP ; raw
    }
    goto reset
}
";
    let output = tally(source).unwrap();
    assert_eq!(
        output.asm,
        "\
; Layout
;
; M1-M2  count
; M3     spare
;
; Notes go into the header.

reset:
        ; Clear the count.
        LDI  A, 0
        ST   M1             ; tens
        JMP  countdown
countdown:
        NOP
here:   NOP ; raw
        JMP  reset
"
    );
}

#[test]
fn key_literals_print_in_hex_and_numbers_keep_their_radix() {
    let source = "\
firmware \"T\" for tally2
entry main {
    a = KEY
    match a {
        '+' => goto main
        0x0B => goto main
        11 => goto main
        0 to 0x09 => goto main
    }
}
";
    let output = tally(source).unwrap();
    assert!(
        output
            .asm
            .contains("CPI  0x0A\n        JZ   main\n        CPI  0x0B\n        JZ   main\n        CPI  11\n        JZ   main\n        CPI  0x0A\n        JC   main\n"),
        "{}",
        output.asm
    );
}

#[test]
fn key_literals_use_the_codes_of_the_board() {
    let source = "\
firmware \"T\" for tally2
entry main {
    a = KEY
    match a {
        '+' => goto main
        'C' => goto main
    }
}
";
    let chip = ChipSpec::Calco(ChipInfo {
        revision: Revision::B,
        date_code: DateCode::new(76, 30),
    });
    let selection = Selection {
        flavor: Flavor::Fixed,
        switches: &[],
    };
    for rev in BoardRev::ALL {
        let board = DeviceKind::Tally2.board(rev);
        let output = compile(source, &Target::new(chip, board), &selection).unwrap();
        for key in ['+', 'C'] {
            let code = board.key_code(key).unwrap();
            let check = format!("CPI  0x{code:02X}\n");
            assert!(output.asm.contains(&check), "{rev:?} {key}\n{}", output.asm);
        }
    }
}

#[test]
fn annunciator_names_use_the_bits_of_the_board() {
    let source = "\
firmware \"S\" for summa6
entry main {
    ANN = ann(minus)
    ANN = ann(minus, error)
    wait
}
";
    let chip = ChipSpec::Calco(ChipInfo {
        revision: Revision::B,
        date_code: DateCode::new(76, 30),
    });
    let selection = Selection {
        flavor: Flavor::Fixed,
        switches: &[],
    };
    for rev in BoardRev::ALL {
        let board = DeviceKind::Summa6.board(rev);
        let output = compile(source, &Target::new(chip, board), &selection).unwrap();
        let bit = |a| 1 << board.ann_bit(a).unwrap();
        let minus = bit(Annunciator::Minus);
        let both = minus | bit(Annunciator::Error);
        let expected = format!(
            "LDI  A, {minus}\n        OUT  ANN\n        LDI  A, {both}\n        OUT  ANN\n"
        );
        assert!(output.asm.contains(&expected), "{rev:?}\n{}", output.asm);
    }
    let tally = "firmware \"T\" for tally2\nentry main {\n    ANN = ann(minus)\n}\n";
    let error = common::error(build(
        tally,
        Revision::B,
        DeviceKind::Tally2,
        Flavor::Fixed,
        &[],
    ));
    assert_eq!(error.message, "tally2 has no annunciator 'minus'");
}

#[test]
fn lock_lamps_use_the_bits_of_the_board() {
    let source = "firmware \"L\" for lock4
entry main {
    ANN = ann(open, shut)
    wait
}
";
    let chip = ChipSpec::Calco(ChipInfo {
        revision: Revision::B,
        date_code: DateCode::new(76, 30),
    });
    let selection = Selection {
        flavor: Flavor::Fixed,
        switches: &[],
    };
    for rev in BoardRev::ALL {
        let board = DeviceKind::Lock4.board(rev);
        let asm = compile(source, &Target::new(chip, board), &selection)
            .unwrap()
            .asm;
        let bit = |a| 1 << board.ann_bit(a).unwrap();
        let both = bit(Annunciator::Open) | bit(Annunciator::Shut);
        let expected = format!(
            "LDI  A, {both}
        OUT  ANN
"
        );
        assert!(
            asm.contains(&expected),
            "{rev:?}
{asm}"
        );
    }
}

#[test]
fn if_else_names_its_labels_after_the_target() {
    let source = "\
firmware \"T\" for tally2
cell n at M1
entry main {
    wait
    a = n
    if zero {
        a = 1
    } else {
        a = 2
    }
    n = a
    goto main
}
";
    let output = tally(source).unwrap();
    assert!(
        output.asm.contains(
            "        JZ   mskip\n        LDI  A, 2\n        JMP  mstore\nmskip:  LDI  A, 1\nmstore: ST   M1\n"
        ),
        "{}",
        output.asm
    );
}
