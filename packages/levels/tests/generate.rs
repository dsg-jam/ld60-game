use std::collections::HashSet;
use std::ops::Range;
use std::sync::LazyLock;

use ld60_calco::{ChipInfo, DateCode, Revision};
use ld60_devices::{ChipSpec, DeviceKind, Rom, RunError};
use ld60_levels::{Generated, Level, Operator, Reference, Symptom, TestCase, initial_rom, run};

const LEVELS: &str = concat!(env!("LD60_ASSETS"), "/levels");
const SEEDS: Range<u64> = 0..200;

static TALLY2: LazyLock<[Reference; 2]> =
    LazyLock::new(|| [Revision::A, Revision::B].map(|rev| reference(DeviceKind::Tally2, rev)));
static SUMMA6: LazyLock<Reference> = LazyLock::new(|| reference(DeviceKind::Summa6, Revision::B));

fn reference(device: DeviceKind, revision: Revision) -> Reference {
    compile(device, revision, DateCode::new(76, 30))
}

fn compile(device: DeviceKind, revision: Revision, date_code: DateCode) -> Reference {
    let path = format!("{LEVELS}/{}", Reference::source(device));
    let source = std::fs::read_to_string(path).unwrap();
    let chip = ChipSpec::Calco(ChipInfo {
        revision,
        date_code,
    });
    Reference::compile(device, chip, &source).unwrap()
}

/// What `rom` shows after `keys` on seed 1.
fn shows(level: &Level, rom: &Rom, keys: &str) -> Result<String, RunError> {
    let level = Level {
        tests: vec![TestCase {
            keys: keys.into(),
            display: String::new(),
        }],
        ..level.clone()
    };
    run(&level, rom, &[1]).unwrap().outcomes[0].actual.clone()
}

/// Whether `symptom` matches what `rom` shows in `case`.
fn matches(symptom: Symptom, level: &Level, rom: &Rom, case: &TestCase) -> bool {
    let shown = shows(level, rom, &case.keys);
    let keys: Vec<char> = case.keys.chars().filter(|c| !c.is_whitespace()).collect();
    let before = keys
        .split_last()
        .map(|(_, rest)| shows(level, rom, &rest.iter().collect::<String>()));
    match symptom {
        Symptom::Freeze => shown.is_err(),
        Symptom::NoResponse => shown.is_ok() && before.is_some_and(|before| before == shown),
        Symptom::WrongAfterClear => {
            shown.is_ok() && keys.iter().any(|key| key.eq_ignore_ascii_case(&'C'))
        }
        Symptom::WrongDigit | Symptom::WrongAnnunciator => shown.is_ok(),
    }
}

/// Lines only in `fixed` and lines only in `buggy`, by longest common subsequence.
fn diff(fixed: &str, buggy: &str) -> (usize, usize) {
    let (a, b): (Vec<&str>, Vec<&str>) = (fixed.lines().collect(), buggy.lines().collect());
    let mut lcs = vec![vec![0; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let common = lcs[0][0];
    (a.len() - common, b.len() - common)
}

/// Checks every seed and returns the operators and symptoms used.
fn check(name: &str, reference: impl Fn(u64) -> &'static Reference) -> HashSet<String> {
    let mut failures = Vec::new();
    let mut seen = HashSet::new();
    for seed in SEEDS {
        let difficulty = (seed % 12) as u32;
        let job = match Generated::new(reference(seed), seed, difficulty) {
            Ok(job) => job,
            Err(error) => {
                failures.push(format!("{name} seed {seed}: {error}"));
                continue;
            }
        };
        let variant = &job.variant;
        if let Err(error) = variant.verify() {
            failures.push(format!("{name} seed {seed}: {error}"));
        }
        let bugs = job.bugs.len();
        if bugs != (1 + difficulty as usize / 4).min(3) {
            failures.push(format!("{name} seed {seed}: {bugs} bugs"));
        }
        let (removed, added) = diff(&variant.fixed, &variant.buggy);
        if removed > bugs || added > bugs || removed + added == 0 {
            failures.push(format!(
                "{name} seed {seed}: {removed} lines removed, {added} added for {bugs} bugs\n{}",
                variant.buggy
            ));
        }
        let keys: HashSet<String> = variant
            .level
            .tests
            .iter()
            .map(|t| t.keys.split_whitespace().collect())
            .collect();
        if keys.len() != variant.level.tests.len() {
            failures.push(format!("{name} seed {seed}: tests repeat keys"));
        }
        let rom = initial_rom(&variant.level, &variant.buggy).unwrap();
        let report = run(&variant.level, &rom, &[1]).unwrap();
        for bug in &job.bugs {
            if report.outcomes[bug.test].passed() {
                failures.push(format!("{name} seed {seed}: test {} passes", bug.test));
            }
            let case = &variant.level.tests[bug.test];
            if !matches(bug.symptom, &variant.level, &rom, case) {
                failures.push(format!(
                    "{name} seed {seed}: '{}' does not show {:?}",
                    case.keys, bug.symptom
                ));
            }
            seen.insert(format!("{:?}", bug.operator));
            seen.insert(format!("{:?}", bug.symptom));
        }
        if report.outcomes.iter().filter(|o| o.passed()).count() < 3 {
            failures.push(format!("{name} seed {seed}: fewer than 3 passing tests"));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    seen
}

#[test]
fn tally2_jobs_verify_and_undo_one_line_per_bug() {
    let seen = check("tally2", |seed| &TALLY2[(seed % 2) as usize]);
    assert!(seen.len() >= 6, "{seen:?}");
}

#[test]
fn summa6_jobs_verify_and_undo_one_line_per_bug() {
    let seen = check("summa6", |_| &SUMMA6);
    for operator in Operator::ALL {
        assert!(seen.contains(&format!("{operator:?}")), "{seen:?}");
    }
    let symptoms = Symptom::ALL
        .iter()
        .filter(|s| seen.contains(&format!("{s:?}")))
        .count();
    assert!(symptoms >= 3, "{seen:?}");
}

#[test]
fn same_seed_gives_same_job() {
    let reference = &*SUMMA6;
    let first = Generated::new(reference, 7, 5).unwrap();
    assert_eq!(Generated::new(reference, 7, 5).unwrap(), first);
    assert_ne!(Generated::new(reference, 8, 5).unwrap(), first);
}

#[test]
fn moves_jobs_to_every_board() {
    let boards: HashSet<_> = (0..16)
        .map(|seed| {
            Generated::new(&SUMMA6, seed, 0)
                .unwrap()
                .variant
                .level
                .board
        })
        .collect();
    assert_eq!(boards.len(), 3);
}

#[test]
fn needs_many_attempts_for_three_bugs() {
    let cases = [
        (0, 203),
        (0, 994),
        (0, 1072),
        (0, 1433),
        (0, 1589),
        (1, 1642),
    ];
    for (rev, seed) in cases {
        let job = Generated::new(&TALLY2[rev], seed, 8).unwrap();
        assert_eq!(job.bugs.len(), 3, "seed {seed}");
    }
}

#[test]
fn compiles_for_every_supported_chip() {
    for device in [DeviceKind::Tally2, DeviceKind::Summa6] {
        for &revision in Reference::revisions(device) {
            for date_code in [DateCode::new(76, 12), DateCode::new(76, 30)] {
                let reference = compile(device, revision, date_code);
                Generated::new(&reference, 0, 8)
                    .unwrap()
                    .variant
                    .verify()
                    .unwrap();
            }
        }
    }
}
