use ld60_calco::{ChipInfo, DateCode, Revision};
use ld60_devices::{ChipSpec, IsaKind};

fn calco(revision: Revision) -> ChipSpec {
    ChipSpec::Calco(ChipInfo {
        revision,
        date_code: DateCode::new(76, 30),
    })
}

#[test]
fn isa_follows_the_revision() {
    let rev_a = calco(Revision::A).isa();
    let rev_b = calco(Revision::B).isa();
    assert_eq!(rev_a.name(), "CalCo Rev A");
    assert_eq!(rev_b.name(), "CalCo Rev B");
    assert!(!rev_a.mnemonics().contains(&"CALL"));
    assert!(rev_b.mnemonics().contains(&"CALL"));
    assert!(rev_a.assemble("CALL 0").is_err());
    assert!(rev_b.assemble("CALL 0").is_ok());
}

#[test]
fn every_revision_is_the_same_family() {
    for revision in Revision::ALL {
        assert_eq!(calco(revision).isa_kind(), IsaKind::Calco);
    }
}
