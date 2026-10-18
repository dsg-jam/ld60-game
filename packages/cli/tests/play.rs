use ld60_cli::{Session, find_level};

fn play(level: &str, fixed: bool, input: &str) -> String {
    let mut session = Session::new(find_level(level).unwrap(), fixed, 1).unwrap();
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
