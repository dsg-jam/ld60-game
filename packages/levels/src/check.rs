use std::fmt;

use ld60_devices::{AnyDevice, Device, MAX_STEPS, Rom, RunError};

use crate::{Level, LevelError, TestCase, device};

pub const TEST_SEEDS: &[u64] = &[1, 2, 3, 4, 5, 6, 7, 8];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub case: TestCase,
    pub actual: Result<String, RunError>,
    pub seed: u64,
}

impl Outcome {
    pub fn passed(&self) -> bool {
        self.actual.as_deref() == Ok(self.case.display.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub outcomes: Vec<Outcome>,
}

impl Report {
    pub fn passed(&self) -> bool {
        self.outcomes.iter().all(Outcome::passed)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for o in &self.outcomes {
            let keys = &o.case.keys;
            let expected = &o.case.display;
            match &o.actual {
                _ if o.passed() => writeln!(f, "PASS '{keys}' shows {expected}")?,
                Ok(actual) => writeln!(
                    f,
                    "FAIL '{keys}' expected {expected}, got {actual} (seed {})",
                    o.seed
                )?,
                Err(error) => writeln!(f, "FAIL '{keys}': {error} (seed {})", o.seed)?,
            }
        }
        Ok(())
    }
}

/// Runs every test case once per seed. A case passes only if all seeds pass.
pub fn run(level: &Level, rom: &Rom, seeds: &[u64]) -> Result<Report, LevelError> {
    let seeds = if seeds.is_empty() { &[0][..] } else { seeds };
    let mut device = device(level, rom.clone())?;
    let outcomes = level
        .tests
        .iter()
        .map(|case| {
            let mut last = None;
            for &seed in seeds {
                let outcome = Outcome {
                    case: case.clone(),
                    actual: run_case(&mut device, case, seed),
                    seed,
                };
                if !outcome.passed() {
                    return outcome;
                }
                last = Some(outcome);
            }
            last.expect("at least one seed")
        })
        .collect();
    Ok(Report { outcomes })
}

fn run_case(device: &mut AnyDevice, case: &TestCase, seed: u64) -> Result<String, RunError> {
    device.power_on(seed);
    device.run_until_halt(MAX_STEPS)?;
    for key in case.keys.chars().filter(|c| !c.is_whitespace()) {
        device.press_key(key)?;
        device.run_until_halt(MAX_STEPS)?;
    }
    Ok(device.readout().to_string())
}
