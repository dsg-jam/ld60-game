use std::path::{Path, PathBuf};

use ld60_levels::DATA_DIR;

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

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ld60-generate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(Path::new(DATA_DIR), &dir);
    dir
}

#[test]
fn check_passes_on_the_committed_files() {
    let (current, _) = run(Path::new(DATA_DIR), true);
    assert!(current);
}

#[test]
fn check_catches_stale_files_and_gen_fixes_them() {
    let dir = scratch();
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
