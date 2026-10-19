use std::path::{Path, PathBuf};

use ld60_cli::{Session, find_variant};
use ld60_devices::DeviceKind;

fn folder(tag: &str) -> PathBuf {
    let name = format!("ld60-procedural-{tag}-{}", std::process::id());
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn generate(device: DeviceKind, seed: u64, job: u32, tag: &str) -> (PathBuf, String) {
    let dir = folder(tag);
    let mut out = Vec::new();
    ld60_cli::procedural::run(&mut out, device, seed, job, &dir).unwrap();
    (dir, String::from_utf8(out).unwrap())
}

fn play(dir: &Path, fixed: bool, input: &str) -> String {
    let variant = find_variant(dir.to_str().unwrap(), None, None).unwrap();
    let mut session = Session::new(variant, fixed, 1).unwrap();
    let mut out = Vec::new();
    ld60_cli::play::run(&mut session, input.as_bytes(), &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn generated_jobs_play_broken_and_pass_when_fixed() {
    for device in DeviceKind::ALL {
        for job in [1, 5, 10] {
            let (dir, summary) = generate(device, 7, job, &format!("{device:?}-{job}"));
            assert!(summary.contains("Bug: "), "{summary}");
            assert!(!summary.contains("job-complaint"), "{summary}");

            let buggy = play(&dir, false, "t\nq\n");
            assert!(buggy.contains("FAIL"), "{buggy}");
            assert!(!buggy.contains(".complaint"), "{buggy}");
            let fixed = play(&dir, true, "t\nq\n");
            assert!(!fixed.contains("FAIL"), "{fixed}");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

#[test]
fn same_arguments_write_the_same_job() {
    let (first, _) = generate(DeviceKind::Summa6, 3, 4, "first");
    let (second, _) = generate(DeviceKind::Summa6, 3, 4, "second");
    for name in ["level.ron", "buggy.asm", "fixed.asm"] {
        assert_eq!(
            std::fs::read_to_string(first.join(name)).unwrap(),
            std::fs::read_to_string(second.join(name)).unwrap(),
            "{name}"
        );
    }
    let _ = std::fs::remove_dir_all(&first);
    let _ = std::fs::remove_dir_all(&second);
}
