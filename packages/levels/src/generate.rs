mod probe;
mod stuck;
mod symptom;

use std::ops::RangeInclusive;
use std::slice;

use ld60_calco::{RAM_LEN, Revision};
use ld60_devices::{BoardRev, ChipSpec, DeviceKind, Fault, Rom, RomKind};
use ld60_dsl::{Flavor, Selection, Target};
use ld60_machine::StuckBit;

use self::probe::Probe;
pub use self::symptom::Symptom;
use crate::check::run_cases;
use crate::difficulty::MAX_SYMPTOM_KEYS;
use crate::mutate::{Mutation, Operator};
use crate::variant::rng::Rng;
use crate::{
    Difficulty, GenerateError, JobText, Level, RomSpec, TEST_SEEDS, TestCase, Variant, initial_rom,
    run_line_tests,
};

/// Tries with fresh bugs before [`Generated::new`] gives up.
const ATTEMPTS: u64 = 256;
/// Operators tried per attempt.
const PICKS: usize = 24;
/// Board rewrites tried before the job stays on board A.
const REWRITES: u64 = 8;
/// Random key sequences tried as passing tests after the end of line tests.
const RANDOM_TESTS: usize = 32;
const MAX_RANDOM_KEYS: u64 = 5;
/// Stuck RAM bits tried before the board stays without one.
const FAULT_TRIES: usize = 16;

/// The working firmware of a device model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    device: DeviceKind,
    chip: ChipSpec,
    asm: String,
}

/// A repair job made from a [`Reference`] with one or more bugs added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generated {
    pub variant: Variant,
    pub bugs: Vec<Bug>,
}

/// One bug in a [`Generated`] job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bug {
    pub operator: Operator,
    /// The first line of the fixed firmware it changes, counted from 0.
    pub line: usize,
    pub symptom: Symptom,
    /// The index of the test in the level that shows it.
    pub test: usize,
}

impl Reference {
    /// The firmware source of `device`, relative to the folder of all levels.
    pub fn source(device: DeviceKind) -> &'static str {
        match device {
            DeviceKind::Tally2 => "tally2/firmware-1.0.fw",
            DeviceKind::Summa6 => "summa6/firmware.fw",
            DeviceKind::Lock4 => "lock4/firmware.fw",
            DeviceKind::Score5 => "score5/firmware.fw",
        }
    }

    /// The `fix` switches of [`Reference::source`] that the working firmware has.
    ///
    /// The Summa6 shows negative totals with the minus light and works
    /// around the `DS` errata, so it works on every date code.
    pub fn fixes(device: DeviceKind) -> &'static [&'static str] {
        match device {
            DeviceKind::Tally2 | DeviceKind::Lock4 | DeviceKind::Score5 => &[],
            DeviceKind::Summa6 => &["negative", "ds-errata"],
        }
    }

    /// The chip revisions that [`Reference::compile`] supports for `device`.
    ///
    /// The Summa6 and the Score5 need instructions that Rev A does not
    /// have. The Lock4 also needs `CMP` from Rev C.
    pub fn revisions(device: DeviceKind) -> &'static [Revision] {
        match device {
            DeviceKind::Tally2 => &Revision::ALL,
            DeviceKind::Summa6 | DeviceKind::Score5 => &[Revision::B, Revision::C],
            DeviceKind::Lock4 => &[Revision::C],
        }
    }

    /// Compiles `source` for `chip` on board A with the [`Reference::fixes`]
    /// of `device`.
    pub fn compile(
        device: DeviceKind,
        chip: ChipSpec,
        source: &str,
    ) -> Result<Self, GenerateError> {
        let target = Target::new(chip, device.board(BoardRev::A));
        let switches: Vec<String> = Self::fixes(device).iter().map(|&s| s.into()).collect();
        let selection = Selection {
            flavor: Flavor::Fixed,
            switches: &switches,
        };
        let output =
            ld60_dsl::compile(source, &target, &selection).map_err(GenerateError::Compile)?;
        let reference = Self {
            device,
            chip,
            asm: output.asm,
        };
        let level = reference.level(0);
        let report = run_line_tests(&level, &initial_rom(&level, &reference.asm)?, TEST_SEEDS)?;
        if !report.passed() {
            return Err(GenerateError::Broken(report));
        }
        Ok(reference)
    }

    pub fn device(&self) -> DeviceKind {
        self.device
    }

    pub fn chip(&self) -> ChipSpec {
        self.chip
    }

    /// The assembly for board A.
    pub fn asm(&self) -> &str {
        &self.asm
    }

    /// A job for this device with no tests yet, on board A.
    fn level(&self, seed: u64) -> Level {
        Level {
            id: Generated::ID.into(),
            title: Generated::ID.into(),
            device: self.device,
            board: BoardRev::A,
            boards: Vec::new(),
            keep_layout: false,
            chip: self.chip,
            unit: format!("{:05}", Rng::new(seed, "unit").next_u64() % 100_000),
            remarks: Vec::new(),
            rom: RomSpec {
                kind: RomKind::Eprom,
                blank_available: true,
            },
            faults: Vec::new(),
            tests: Vec::new(),
            reward: 0,
            firmware: None,
            text: None,
        }
    }
}

impl Generated {
    /// The [`Level::id`] of every generated job.
    pub const ID: &str = "generated";

    /// Makes the job for `seed` with the bugs and faults of `difficulty`.
    ///
    /// The `reference` should be compiled for [`Difficulty::chip`]. The first
    /// bug shows within [`Difficulty::symptom_keys`] keys and the others
    /// within its end. If no such bug is found, any length is allowed. If no
    /// job with [`Difficulty::bugs`] bugs is found, the job gets fewer.
    ///
    /// The job moves to a board picked by the seed. Then each bug changes
    /// one line, and a test shows each bug with the fewest keys. The other
    /// tests pass on the buggy firmware. The job always passes
    /// [`Variant::verify`].
    pub fn new(
        reference: &Reference,
        seed: u64,
        difficulty: &Difficulty,
    ) -> Result<Self, GenerateError> {
        Self::avoiding(reference, seed, difficulty, None)
    }

    /// Like [`Generated::new`], but not on board `avoid`.
    pub fn avoiding(
        reference: &Reference,
        seed: u64,
        difficulty: &Difficulty,
        avoid: Option<BoardRev>,
    ) -> Result<Self, GenerateError> {
        let working = Self::working(reference, seed, difficulty.fault_chance(), avoid);
        let lengths = [difficulty.symptom_keys().clone(), 0..=MAX_SYMPTOM_KEYS];
        let mut rng = Rng::new(seed, "stuck");
        if (rng.below(100) as u32) < difficulty.stuck_chance()
            && let Some(job) = lengths
                .iter()
                .find_map(|keys| Self::stuck(&working, &mut rng, difficulty, keys))
        {
            return Ok(job.with_text(seed));
        }
        let job = (1..=difficulty.bugs())
            .rev()
            .find_map(|bugs| {
                lengths.iter().find_map(|keys| {
                    (0..ATTEMPTS).find_map(|attempt| {
                        let mut rng = Rng::new(seed, &format!("bug {attempt}"));
                        Self::attempt(&working, &mut rng, difficulty, bugs, keys)
                    })
                })
            })
            .ok_or(GenerateError::NoBug)?;
        Ok(job.with_text(seed))
    }

    /// The reference moved to the board for `seed`, as a job that keeps it
    /// there. With `fault_chance` percent, the board gets a stuck RAM bit.
    fn working(
        reference: &Reference,
        seed: u64,
        fault_chance: u32,
        avoid: Option<BoardRev>,
    ) -> Variant {
        let original = Variant {
            level: reference.level(seed),
            buggy: reference.asm.clone(),
            fixed: reference.asm.clone(),
        };
        let boards: Vec<BoardRev> = BoardRev::ALL
            .into_iter()
            .filter(|&board| Some(board) != avoid)
            .collect();
        let board = boards[Rng::new(seed, "board").below(boards.len())];
        let mut working = (0..REWRITES)
            .filter_map(|n| original.rewrite(board, seed.wrapping_add(n)))
            .find(|v| v.behaves_like(&original) && v.passes_line_tests())
            .unwrap_or(original);
        working.level.boards = vec![working.level.board];
        working.level.keep_layout = true;
        let mut rng = Rng::new(seed, "fault");
        if (rng.below(100) as u32) < fault_chance {
            working = working.with_harmless_fault(&mut rng);
        }
        working
    }

    fn attempt(
        working: &Variant,
        rng: &mut Rng,
        difficulty: &Difficulty,
        bugs: usize,
        keys: &RangeInclusive<usize>,
    ) -> Option<Self> {
        let level = &working.level;
        let lines: Vec<String> = working.fixed.lines().map(String::from).collect();
        let fixed = initial_rom(level, &working.fixed).ok()?;
        let key_codes: Vec<u8> = level
            .board()
            .keys
            .iter()
            .filter(|(key, _)| !key.is_ascii_digit())
            .map(|&(_, code)| code)
            .collect();
        let mut chosen: Vec<(Mutation, TestCase)> = Vec::new();
        for _ in 0..PICKS {
            if chosen.len() == bugs {
                break;
            }
            let operator = difficulty.operator(rng);
            let candidates: Vec<Mutation> = Mutation::candidates(&lines, operator, &key_codes)
                .into_iter()
                .filter(|m| chosen.iter().all(|(other, ..)| other.apart(m)))
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let mutation = candidates[rng.below(candidates.len())].clone();
            let mutant = Mutation::apply(&lines, &[&mutation]);
            let Ok(mutant_rom) = initial_rom(level, &mutant) else {
                continue;
            };
            let Some(probe) = Probe::find(level, &working.fixed, &mutant, *keys.end()) else {
                continue;
            };
            let shortest = if chosen.is_empty() { *keys.start() } else { 0 };
            if !(shortest..=*keys.end()).contains(&probe.keys.len()) {
                continue;
            }
            if chosen.iter().any(|(_, c)| same_keys(&c.keys, &probe.keys)) {
                continue;
            }
            let Some(case) = TestCase::shown(level, &fixed, &probe.keys) else {
                continue;
            };
            if case.fails_everywhere(level, &mutant_rom) {
                chosen.push((mutation, case));
            }
        }
        if chosen.len() < bugs {
            return None;
        }
        let mutations: Vec<&Mutation> = chosen.iter().map(|(m, ..)| m).collect();
        let buggy = Mutation::apply(&lines, &mutations);
        let buggy_rom = initial_rom(level, &buggy).ok()?;
        let mut tests: Vec<TestCase> = chosen.iter().map(|(_, case)| case.clone()).collect();
        if !tests
            .iter()
            .all(|case| case.fails_everywhere(level, &buggy_rom))
        {
            return None;
        }
        let symptoms: Vec<Symptom> = tests
            .iter()
            .map(|case| Symptom::shown(level, &fixed, &buggy_rom, &case.keys))
            .collect::<Option<_>>()?;
        if !TestCase::add_passing(level, &fixed, &buggy_rom, &mut tests, rng) {
            return None;
        }
        let mut level = level.clone();
        level.tests = tests;
        level.reward = difficulty.reward(bugs);
        let variant = Variant {
            level,
            buggy,
            fixed: working.fixed.clone(),
        };
        variant.verify().ok()?;
        let bugs = chosen
            .into_iter()
            .zip(symptoms)
            .enumerate()
            .map(|(test, ((mutation, _), symptom))| Bug {
                operator: mutation.operator,
                line: mutation.line,
                symptom,
                test,
            })
            .collect();
        Some(Self { variant, bugs })
    }

    /// Picks the customer and how they describe the first bug. The other
    /// bugs get a remark each.
    fn with_text(mut self, seed: u64) -> Self {
        let mut rng = Rng::new(seed, "text");
        let level = &mut self.variant.level;
        level.text = Some(JobText {
            customer: JobText::CUSTOMERS[rng.below(JobText::CUSTOMERS.len())].into(),
            symptom: self.bugs[0].symptom,
            take: 1 + rng.below(JobText::TAKES as usize) as u32,
        });
        for bug in &self.bugs[1..] {
            let remark = JobText::also(bug.symptom);
            if !level.remarks.contains(&remark) {
                level.remarks.push(remark);
            }
        }
        self
    }
}

impl TestCase {
    /// The test for `keys` that `rom` passes on every test seed.
    fn shown(level: &Level, rom: &Rom, keys: &str) -> Option<Self> {
        let probe = Self {
            keys: keys.into(),
            display: String::new(),
        };
        let report = run_cases(level, rom, slice::from_ref(&probe), &TEST_SEEDS[..1]).ok()?;
        let display = report.outcomes.first()?.actual.clone().ok()?;
        let case = Self {
            keys: keys.into(),
            display,
        };
        let report = run_cases(level, rom, slice::from_ref(&case), TEST_SEEDS).ok()?;
        report.passed().then_some(case)
    }

    /// Adds 3 or 4 tests that `fixed` and `buggy` both pass to `tests`.
    /// Returns whether enough were found.
    fn add_passing(
        level: &Level,
        fixed: &Rom,
        buggy: &Rom,
        tests: &mut Vec<Self>,
        rng: &mut Rng,
    ) -> bool {
        let wanted = tests.len() + 3 + rng.below(2);
        let line_tests = level.device.line_tests().iter().map(|t| t.keys.to_owned());
        let random = (0..RANDOM_TESTS).map(|_| random_keys(level, rng));
        for keys in line_tests.chain(random.collect::<Vec<_>>()) {
            if tests.len() == wanted {
                break;
            }
            if tests.iter().any(|t| same_keys(&t.keys, &keys)) {
                continue;
            }
            let Some(case) = Self::shown(level, fixed, &keys) else {
                continue;
            };
            let report = run_cases(level, buggy, slice::from_ref(&case), TEST_SEEDS);
            if report.is_ok_and(|r| r.passed()) {
                tests.push(case);
            }
        }
        tests.len() == wanted
    }

    /// Whether `rom` fails this test on every test seed, so the bug always shows.
    fn fails_everywhere(&self, level: &Level, rom: &Rom) -> bool {
        TEST_SEEDS.iter().all(|&seed| {
            run_cases(level, rom, slice::from_ref(self), &[seed]).is_ok_and(|r| !r.passed())
        })
    }
}

impl Variant {
    /// The job with a stuck RAM bit that changes nothing the customer sees,
    /// or the job as it is if no such bit is found.
    fn with_harmless_fault(self, rng: &mut Rng) -> Self {
        (0..FAULT_TRIES)
            .find_map(|_| {
                let bit = StuckBit {
                    cell: rng.below(RAM_LEN) as u8,
                    bit: rng.below(4) as u8,
                    high: rng.below(2) == 1,
                };
                let mut faulty = self.clone();
                faulty.level.faults.push(Fault::StuckRamBit(bit));
                faulty.level.remarks.push(JobText::FAULT_REMARK.into());
                (faulty.behaves_like(&self) && faulty.passes_line_tests()).then_some(faulty)
            })
            .unwrap_or(self)
    }

    fn passes_line_tests(&self) -> bool {
        initial_rom(&self.level, &self.fixed)
            .and_then(|rom| run_line_tests(&self.level, &rom, TEST_SEEDS))
            .is_ok_and(|report| report.passed())
    }
}

fn random_keys(level: &Level, rng: &mut Rng) -> String {
    let keys = level.board().keys;
    let len = 1 + rng.next_u64() % MAX_RANDOM_KEYS;
    (0..len).map(|_| keys[rng.below(keys.len())].0).collect()
}

fn same_keys(a: &str, b: &str) -> bool {
    a.chars()
        .filter(|c| !c.is_whitespace())
        .eq(b.chars().filter(|c| !c.is_whitespace()))
}
