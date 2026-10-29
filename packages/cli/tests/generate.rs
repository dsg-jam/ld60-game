use std::path::{Path, PathBuf};

use ld60_cli::LEVELS;

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn run(data_dir: &Path, check: bool) -> (bool, String) {
    let mut out = Vec::new();
    let current = ld60_cli::generate::run(&mut out, data_dir, check).unwrap();
    (current, String::from_utf8(out).unwrap())
}

fn scratch(tag: &str) -> PathBuf {
    let name = format!("ld60-generate-{tag}-{}", std::process::id());
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(Path::new(LEVELS), &dir);
    dir
}

#[test]
fn check_passes_on_the_committed_files() {
    let (current, _) = run(Path::new(LEVELS), true);
    assert!(current);
}

#[test]
fn check_catches_stale_files_and_gen_fixes_them() {
    let dir = scratch("stale");
    let fixed = dir.join("summa6/02-last-digit/fixed.asm");
    let text = std::fs::read_to_string(&fixed).unwrap();
    std::fs::write(&fixed, text.replace("LDI  B, 6", "LDI  B, 7")).unwrap();

    let (current, out) = run(&dir, true);
    assert!(!current);
    assert!(
        out.contains("summa6/02-last-digit/fixed.asm is stale"),
        "{out}"
    );
    assert!(
        std::fs::read_to_string(&fixed)
            .unwrap()
            .contains("LDI  B, 7")
    );

    let (current, out) = run(&dir, false);
    assert!(!current);
    assert!(
        out.contains("summa6/02-last-digit/fixed.asm written"),
        "{out}"
    );
    assert_eq!(std::fs::read_to_string(&fixed).unwrap(), text);

    let (current, _) = run(&dir, true);
    assert!(current);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failing_level_stops_all_writes() {
    let dir = scratch("fail");
    let fixed = dir.join("tally2/02-back-to-one/fixed.asm");
    std::fs::write(&fixed, "stale").unwrap();
    let source = dir.join("summa6/firmware.fw");
    let text = std::fs::read_to_string(&source).unwrap();
    let broken = format!("{text}\nblock broken {{\n    goto nowhere\n}}\n");
    std::fs::write(&source, broken).unwrap();

    let mut out = Vec::new();
    let error = ld60_cli::generate::run(&mut out, &dir, false).unwrap_err();
    assert!(
        error.to_string().contains("unknown label 'nowhere'"),
        "{error}"
    );
    assert_eq!(std::fs::read_to_string(&fixed).unwrap(), "stale");
    let _ = std::fs::remove_dir_all(&dir);
}
