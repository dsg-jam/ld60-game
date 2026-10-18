use ld60_cli::{Session, find_level};
use ld60_levels::Variant;

fn play(level: &str, fixed: bool, input: &str) -> String {
    play_variant(
        Variant::original(find_level(level).unwrap()).unwrap(),
        fixed,
        input,
    )
}

fn play_variant(variant: Variant, fixed: bool, input: &str) -> String {
    let mut session = Session::new(variant, fixed, 1).unwrap();
    let mut out = Vec::new();
    ld60_cli::play::run(&mut session, input.as_bytes(), &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn keys_drive_the_display() {
    let out = play("summa6/02-last-digit", true, "k 12+34=\nq\n");
    assert!(out.contains("[ 000046 ]"), "{out}");
}

#[test]
fn buggy_firmware_shows_the_symptom() {
    let out = play("summa6/02-last-digit", false, "k 12+34=\nt\n");
    assert!(out.contains("[ 000040 ]"), "{out}");
    assert!(out.contains("FAIL '12+34='"), "{out}");
}

#[test]
fn breakpoints_stop_on_labels() {
    let out = play("summa6/01-strange-symbols", true, "b add\nk 5+\nl\n");
    assert!(out.contains("Breakpoint at 0x"), "{out}");
    assert!(out.contains("add:"), "{out}");
}

#[test]
fn stepping_reports_the_halt() {
    let out = play("summa6/01-strange-symbols", true, "s 3\n");
    assert!(out.contains("Waiting for a key."), "{out}");
}

#[test]
fn variants_keep_the_symptom() {
    let variant = Variant::new(find_level("summa6/02-last-digit").unwrap(), 7).unwrap();
    let out = play_variant(variant, false, "k 12+34=\nq\n");
    assert!(out.contains("[ 000040 ]"), "{out}");
}
