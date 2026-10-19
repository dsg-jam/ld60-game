mod common;

use std::collections::BTreeSet;

use common::run;
use ld60_calco::datasheet::INSTRUCTIONS;
use ld60_calco::{ChipInfo, DateCode, Op, Revision};
use ld60_machine::{AsmErrorKind, Isa};

/// Supported instruction forms in datasheet syntax, like `LD Mn` or `JZ aa`.
fn forms(revision: Revision) -> Vec<&'static str> {
    INSTRUCTIONS
        .iter()
        .filter(|doc| revision.supports(doc.op))
        .map(|doc| doc.syntax)
        .collect()
}

fn chip(revision: Revision) -> ChipInfo {
    ChipInfo {
        revision,
        date_code: DateCode::new(76, 30),
    }
}

fn first_error(revision: Revision, source: &str) -> AsmErrorKind {
    match revision.isa().assemble(source) {
        Ok(_) => panic!("Rev {revision} assembles {source:?}"),
        Err(errors) => errors[0].kind.clone(),
    }
}

fn unsupported_on_a(form: &str) -> AsmErrorKind {
    AsmErrorKind::Unsupported {
        form: form.into(),
        isa: "CalCo Rev A",
    }
}

fn every_op() -> impl Iterator<Item = Op> {
    (0..=u8::MAX).map(|first| Op::decode(first, 0))
}

#[test]
fn rev_a_rejects_ops_it_lacks() {
    for (source, form) in [
        ("CALL 0", "CALL"),
        ("call 0", "CALL"),
        ("JNZ 0", "JNZ"),
        ("OUT ANN", "OUT"),
        ("LDI B, 5", "LDI B, 0x05"),
        ("LD [B+2]", "LD [B+2]"),
        ("st [b]", "ST [B]"),
    ] {
        assert_eq!(
            first_error(Revision::A, source),
            unsupported_on_a(form),
            "{source}"
        );
    }
}

#[test]
fn unknown_and_unsupported_differ() {
    let unknown = AsmErrorKind::UnknownMnemonic {
        mnemonic: "FOO".into(),
    };
    assert_eq!(first_error(Revision::A, "FOO"), unknown);
    assert_eq!(first_error(Revision::B, "FOO"), unknown);
    assert_eq!(first_error(Revision::A, "DJNZ 0"), unsupported_on_a("DJNZ"));
}

#[test]
fn unsupported_reads_like_the_datasheet() {
    assert_eq!(
        unsupported_on_a("LD [B+2]").to_string(),
        "'LD [B+2]' is not supported by CalCo Rev A"
    );
}

#[test]
fn rev_b_rejects_what_only_rev_c_has() {
    for (source, form) in [("CMP M1", "CMP"), ("cmp [b+2]", "CMP"), ("SWP", "SWP")] {
        assert_eq!(
            first_error(Revision::B, source),
            AsmErrorKind::Unsupported {
                form: form.into(),
                isa: "CalCo Rev B",
            },
            "{source}"
        );
        assert!(Revision::C.isa().assemble(source).is_ok(), "{source}");
    }
}

#[test]
fn each_revision_keeps_the_ops_of_the_one_before() {
    for pair in Revision::ALL.windows(2) {
        let [old, new] = [pair[0], pair[1]];
        for op in every_op().filter(|&op| old.supports(op)) {
            assert!(new.supports(op), "Rev {new} lacks {op} from Rev {old}");
        }
    }
}

#[test]
fn rev_c_runs_every_documented_op() {
    assert!(every_op().all(|op| matches!(op, Op::Reserved(_)) || Revision::C.supports(op)));
}

#[test]
fn later_revisions_take_every_op() {
    let source = "CALL f\nf: LDI B, 2\nLD [B+1]\nOUT ANN\nDJNZ f\nRET";
    for revision in [Revision::B, Revision::C] {
        assert!(revision.isa().assemble(source).is_ok(), "Rev {revision}");
    }
}

#[test]
fn labels_named_like_unsupported_ops_still_work() {
    let source = "CALL: LDI A, 1\nJMP CALL";
    assert!(Revision::A.isa().assemble(source).is_ok());
}

#[test]
fn mnemonics_match_the_supported_ops() {
    for revision in Revision::ALL {
        let supported: BTreeSet<String> = every_op()
            .filter(|&op| revision.supports(op))
            .filter_map(|op| op.to_string().split_whitespace().next().map(String::from))
            .collect();
        let listed: BTreeSet<String> = revision
            .isa()
            .mnemonics()
            .iter()
            .map(|m| m.to_string())
            .collect();
        assert_eq!(listed, supported, "Rev {revision}");
    }
}

#[test]
fn rev_a_has_the_minimal_set() {
    assert_eq!(
        Revision::A.isa().mnemonics(),
        [
            "NOP", "HLT", "INC", "LD", "ST", "LDI", "CPI", "IN", "JMP", "JZ", "JC"
        ]
    );
}

#[test]
fn rev_c_adds_cmp_and_swp() {
    let rev_b: BTreeSet<_> = Revision::B.isa().mnemonics().iter().collect();
    let rev_c: BTreeSet<_> = Revision::C.isa().mnemonics().iter().collect();
    let added: Vec<_> = rev_c.difference(&rev_b).copied().copied().collect();
    assert_eq!(added, ["CMP", "SWP"]);
}

#[test]
fn forms_tell_addressing_modes_apart() {
    assert_eq!(
        forms(Revision::A),
        [
            "NOP",
            "HLT",
            "INC",
            "LD Mn",
            "ST Mn",
            "LDI A, ii",
            "CPI ii",
            "IN p",
            "JMP aa",
            "JZ aa",
            "JC aa"
        ]
    );
    let rev_b = forms(Revision::B);
    for form in ["LD [B+n]", "ADC Mn", "LDI B, ii", "JNC aa", "CALL aa"] {
        assert!(rev_b.contains(&form), "{form}");
    }
    assert!(!rev_b.contains(&"Jcc aa"));
    assert!(!rev_b.contains(&"CMP Mn"));
    let rev_c = forms(Revision::C);
    for form in ["CMP Mn", "CMP [B+n]", "SWP"] {
        assert!(rev_c.contains(&form), "{form}");
    }
}

#[test]
fn rev_a_runs_unsupported_ops_as_reserved() {
    let source = ".db 0x31, 0x07\nHLT";
    let (rev_a, _) = run(source, chip(Revision::A), 100);
    let (rev_b, _) = run(source, chip(Revision::B), 100);
    assert_eq!((rev_a.a(), rev_a.b()), (0, 0));
    assert_eq!((rev_b.a(), rev_b.b()), (0, 7));
}

#[test]
fn rev_a_runs_the_address_byte_of_a_call() {
    let (cpu, _) = run(".db 0xF0, 0x06\nINC\nHLT", chip(Revision::A), 100);
    assert_eq!(cpu.a(), 2, "0x06 is INC");
}

#[test]
fn disassembler_agrees_with_the_cpu() {
    let image = Revision::B.isa().assemble("LDI B, 7\nINC").unwrap().image;
    let text = |revision: Revision| -> Vec<String> {
        revision.isa().disassemble(&image)[..3]
            .iter()
            .map(|line| line.text.clone())
            .collect()
    };
    assert_eq!(text(Revision::A), [".db 0x31", ".db 0x07", "INC"]);
    assert_eq!(text(Revision::B), ["LDI B, 0x07", "INC", ".db 0xFF"]);
}

#[test]
fn in_rev_tells_the_revisions_apart() {
    let ids: Vec<u8> = Revision::ALL
        .into_iter()
        .map(|revision| run("IN REV\nHLT", chip(revision), 100).0.a())
        .collect();
    assert_eq!(ids, [0x10, 0x20, 0x30]);
}
