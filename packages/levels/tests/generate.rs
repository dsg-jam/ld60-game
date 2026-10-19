use std::collections::HashSet;
use std::ops::Range;
use std::sync::LazyLock;

use ld60_calco::{ChipInfo, DateCode, Revision};
use ld60_devices::{ChipSpec, DeviceKind, Fault, Rom, RunError};
use ld60_levels::{
    Difficulty, Generated, JobText, Level, Operator, Reference, Symptom, TestCase, initial_rom, run,
};

const LEVELS: &str = concat!(env!("LD60_ASSETS"), "/levels");
const SEEDS: Range<u64> = 0..200;
/// Job numbers the seeds cycle through.
const JOBS: u64 = 24;

static TALLY2: LazyLock<Vec<Reference>> = LazyLock::new(|| every_chip(DeviceKind::Tally2));
static SUMMA6: LazyLock<Vec<Reference>> = LazyLock::new(|| every_chip(DeviceKind::Summa6));
static LOCK4: LazyLock<Vec<Reference>> = LazyLock::new(|| every_chip(DeviceKind::Lock4));

/// The reference for every chip a [`Difficulty`] picks.
fn every_chip(device: DeviceKind) -> Vec<Reference> {
    let mut references: Vec<Reference> = Vec::new();
    for job in 0..32 {
        let chip = Difficulty::for_job(job, device).chip();
        if references.iter().all(|r| r.chip() != chip) {
            let ChipSpec::Calco(info) = chip;
            references.push(compile(device, info.revision, info.date_code));
        }
    }
    references
}

/// The reference for the chip of `difficulty`.
fn for_chip<'a>(references: &'a [Reference], difficulty: &Difficulty) -> &'a Reference {
    references
        .iter()
        .find(|r| r.chip() == difficulty.chip())
        .unwrap()
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

/// The columns of the first operand and of the comment in `line`.
fn columns(line: &str) -> (Option<usize>, Option<usize>) {
    let comment = line.find(';');
    let code = &line[..comment.unwrap_or(line.len())];
    let start = code.find(':').map_or(0, |colon| colon + 1);
    let body = code[start..].trim_start();
    let operand = body
        .find(' ')
        .map(|space| body[space..].trim_start())
        .filter(|operands| !operands.is_empty())
        .map(|operands| code.len() - operands.len());
    (operand, comment)
}

/// The changed lines of `job` that do not keep the columns of the fixed line.
fn misaligned(job: &Generated) -> Vec<String> {
    let fixed: Vec<&str> = job.variant.fixed.lines().collect();
    let buggy: Vec<&str> = job.variant.buggy.lines().collect();
    let mut bugs = job.bugs.clone();
    bugs.sort_by_key(|bug| bug.line);
    let mut pairs = Vec::new();
    let mut shift = 0;
    for bug in bugs {
        let at = bug.line - shift;
        match bug.operator {
            Operator::OffByOne | Operator::FlippedJump | Operator::WrongCell => {
                pairs.push((fixed[bug.line], buggy[at]));
            }
            Operator::SwappedInstructions => {
                pairs.push((fixed[bug.line + 1], buggy[at]));
                pairs.push((fixed[bug.line], buggy[at + 1]));
            }
            Operator::DroppedInstruction => {
                let code = fixed[bug.line].split(';').next().unwrap_or_default();
                if !code.contains(':') {
                    shift += 1;
                }
            }
        }
    }
    pairs
        .into_iter()
        .filter(|(fixed, buggy)| columns(fixed) != columns(buggy))
        .map(|(fixed, buggy)| format!("'{fixed}' became '{buggy}'"))
        .collect()
}

/// Checks every seed and returns the operators, symptoms, chips and faults used.
fn check(device: DeviceKind, references: &[Reference]) -> HashSet<String> {
    let name = format!("{device:?}");
    let mut failures = Vec::new();
    let mut seen = HashSet::new();
    for seed in SEEDS {
        let difficulty = Difficulty::for_job((seed % JOBS) as u32, device);
        let reference = for_chip(references, &difficulty);
        let job = match Generated::new(reference, seed, &difficulty) {
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
        if bugs != difficulty.bugs() {
            failures.push(format!("{name} seed {seed}: {bugs} bugs"));
        }
        if variant.level.reward != difficulty.reward(bugs) {
            failures.push(format!(
                "{name} seed {seed}: reward {}",
                variant.level.reward
            ));
        }
        if variant.level.chip != difficulty.chip() {
            failures.push(format!("{name} seed {seed}: chip {:?}", variant.level.chip));
        }
        if let Some(text) = &variant.level.text {
            let customer = JobText::CUSTOMERS.contains(&text.customer.as_str());
            let take = (1..=JobText::TAKES).contains(&text.take);
            if !customer || !take || text.symptom != job.bugs[0].symptom {
                failures.push(format!("{name} seed {seed}: {text:?}"));
            }
        } else {
            failures.push(format!("{name} seed {seed}: no text"));
        }
        let faults = variant.level.faults.len();
        let remark = variant
            .level
            .remarks
            .iter()
            .any(|r| r == JobText::FAULT_REMARK);
        if faults > 1 || remark != (faults == 1) {
            failures.push(format!("{name} seed {seed}: {faults} faults"));
        }
        if faults == 1 {
            seen.insert("fault".into());
        }
        seen.insert(format!("{:?}", variant.level.chip));
        let (removed, added) = diff(&variant.fixed, &variant.buggy);
        if removed > bugs || added > bugs || removed + added == 0 {
            failures.push(format!(
                "{name} seed {seed}: {removed} lines removed, {added} added for {bugs} bugs\n{}",
                variant.buggy
            ));
        }
        for line in misaligned(&job) {
            failures.push(format!("{name} seed {seed}: {line}"));
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
        for (index, bug) in job.bugs.iter().enumerate() {
            if report.outcomes[bug.test].passed() {
                failures.push(format!("{name} seed {seed}: test {} passes", bug.test));
            }
            let case = &variant.level.tests[bug.test];
            let length = case.keys.split_whitespace().collect::<String>().len();
            let range = match index {
                0 => difficulty.symptom_keys().clone(),
                _ => 0..=*difficulty.symptom_keys().end(),
            };
            if !range.contains(&length) {
                failures.push(format!(
                    "{name} seed {seed}: '{}' is outside {range:?}",
                    case.keys
                ));
            }
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
    let seen = check(DeviceKind::Tally2, &TALLY2);
    assert!(seen.len() >= 6, "{seen:?}");
}

#[test]
fn summa6_jobs_verify_and_undo_one_line_per_bug() {
    let seen = check(DeviceKind::Summa6, &SUMMA6);
    assert!(seen.contains("fault"), "{seen:?}");
    for reference in SUMMA6.iter() {
        assert!(
            seen.contains(&format!("{:?}", reference.chip())),
            "{seen:?}"
        );
    }
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
fn lock4_jobs_verify_and_undo_one_line_per_bug() {
    let seen = check(DeviceKind::Lock4, &LOCK4);
    assert!(seen.contains("fault"), "{seen:?}");
    for operator in Operator::ALL {
        assert!(seen.contains(&format!("{operator:?}")), "{seen:?}");
    }
    for symptom in [Symptom::WrongAnnunciator, Symptom::WrongAfterClear] {
        assert!(seen.contains(&format!("{symptom:?}")), "{seen:?}");
    }
}

#[test]
fn same_seed_gives_same_job() {
    let difficulty = Difficulty::for_job(5, DeviceKind::Summa6);
    let reference = for_chip(&SUMMA6, &difficulty);
    let first = Generated::new(reference, 7, &difficulty).unwrap();
    assert_eq!(Generated::new(reference, 7, &difficulty).unwrap(), first);
    assert_ne!(Generated::new(reference, 8, &difficulty).unwrap(), first);
}

#[test]
fn moves_jobs_to_every_board() {
    for (device, references) in [(DeviceKind::Summa6, &SUMMA6), (DeviceKind::Lock4, &LOCK4)] {
        let difficulty = Difficulty::for_job(0, device);
        let reference = for_chip(references, &difficulty);
        let boards: HashSet<_> = (0..16)
            .map(|seed| {
                Generated::new(reference, seed, &difficulty)
                    .unwrap()
                    .variant
                    .level
                    .board
            })
            .collect();
        assert_eq!(boards.len(), 3, "{device:?}");
    }
}

#[test]
fn needs_many_attempts_for_three_bugs() {
    let cases = [
        (Revision::A, 203),
        (Revision::A, 994),
        (Revision::A, 1072),
        (Revision::A, 1433),
        (Revision::A, 1589),
        (Revision::B, 1642),
    ];
    let difficulty = Difficulty::for_job(12, DeviceKind::Tally2);
    for (revision, seed) in cases {
        let reference = compile(DeviceKind::Tally2, revision, DateCode::new(76, 30));
        let job = Generated::new(&reference, seed, &difficulty).unwrap();
        assert_eq!(job.bugs.len(), 3, "seed {seed}");
    }
}

#[test]
fn compiles_for_every_supported_chip() {
    for device in DeviceKind::ALL {
        let difficulty = Difficulty::for_job(12, device);
        for &revision in Reference::revisions(device) {
            for date_code in [DateCode::new(76, 12), DateCode::new(76, 30)] {
                let reference = compile(device, revision, date_code);
                Generated::new(&reference, 0, &difficulty)
                    .unwrap()
                    .variant
                    .verify()
                    .unwrap();
            }
        }
    }
}

#[test]
fn first_jobs_show_bugs_with_few_keys_on_the_newest_chip() {
    for device in DeviceKind::ALL {
        let difficulty = Difficulty::for_job(0, device);
        let ChipSpec::Calco(chip) = difficulty.chip();
        assert_eq!(Some(&chip.revision), Reference::revisions(device).last());
        let references = match device {
            DeviceKind::Tally2 => &TALLY2,
            DeviceKind::Summa6 => &SUMMA6,
            DeviceKind::Lock4 => &LOCK4,
        };
        let reference = for_chip(references, &difficulty);
        for seed in 0..20 {
            let job = Generated::new(reference, seed, &difficulty).unwrap();
            let keys = &job.variant.level.tests[0].keys;
            assert!(keys.len() <= 2, "seed {seed}: '{keys}'");
            assert_eq!(job.bugs.len(), 1);
            assert!(job.variant.level.faults.is_empty());
            for bug in &job.bugs {
                assert!(
                    matches!(bug.operator, Operator::OffByOne | Operator::FlippedJump),
                    "{bug:?}"
                );
            }
        }
    }
}

#[test]
fn later_jobs_are_harder_and_pay_more() {
    for device in DeviceKind::ALL {
        let jobs: Vec<Difficulty> = (0..40)
            .map(|job| Difficulty::for_job(job, device))
            .collect();
        for pair in jobs.windows(2) {
            let (earlier, later) = (&pair[0], &pair[1]);
            assert!(earlier.bugs() <= later.bugs());
            assert!(earlier.fault_chance() <= later.fault_chance());
            assert!(earlier.symptom_keys().end() <= later.symptom_keys().end());
            assert!(earlier.reward(1) <= later.reward(1));
            assert!(
                later
                    .weights()
                    .iter()
                    .zip(earlier.weights())
                    .all(|(l, e)| l >= e)
            );
        }
        let last = jobs.last().unwrap();
        assert_eq!(last.bugs(), 3);
        assert!(last.weights().iter().all(|&w| w > 0));
        assert!(last.symptom_keys().start() <= last.symptom_keys().end());
    }
    let tally2 = Difficulty::for_job(10, DeviceKind::Tally2);
    let summa6 = Difficulty::for_job(10, DeviceKind::Summa6);
    assert!(tally2.reward(2) < summa6.reward(2));
}

#[test]
fn late_jobs_pick_old_chips_and_errata() {
    let chips: HashSet<ChipSpec> = (8..40)
        .map(|job| Difficulty::for_job(job, DeviceKind::Summa6).chip())
        .collect();
    let early = ChipSpec::Calco(ChipInfo {
        revision: Revision::B,
        date_code: DateCode::new(76, 12),
    });
    assert!(chips.contains(&early), "{chips:?}");
    for chip in chips {
        let ChipSpec::Calco(info) = chip;
        assert!(Reference::revisions(DeviceKind::Summa6).contains(&info.revision));
    }
}

#[test]
fn faults_change_nothing_the_customer_sees() {
    let difficulty = Difficulty::for_job(30, DeviceKind::Summa6);
    let reference = for_chip(&SUMMA6, &difficulty);
    let faulty: Vec<Generated> = (0..40)
        .map(|seed| Generated::new(reference, seed, &difficulty).unwrap())
        .filter(|job| !job.variant.level.faults.is_empty())
        .collect();
    assert!(!faulty.is_empty());
    for job in faulty {
        let level = &job.variant.level;
        assert!(matches!(level.faults[..], [Fault::StuckRamBit(_)]));
        let clean = Level {
            faults: Vec::new(),
            ..level.clone()
        };
        let fixed = |level: &Level| {
            let rom = initial_rom(level, &job.variant.fixed).unwrap();
            run(level, &rom, &[1, 2, 3]).unwrap()
        };
        assert!(fixed(level).passed());
        assert!(fixed(&clean).passed());
    }
}

#[test]
fn level_files_keep_the_text() {
    let difficulty = Difficulty::for_job(9, DeviceKind::Summa6);
    let reference = for_chip(&SUMMA6, &difficulty);
    let level = Generated::new(reference, 3, &difficulty)
        .unwrap()
        .variant
        .level;
    assert!(level.text.is_some());
    assert_eq!(Level::from_ron(&level.to_ron().unwrap()).unwrap(), level);
}
