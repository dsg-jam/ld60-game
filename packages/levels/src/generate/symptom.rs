use ld60_devices::{AnyDevice, Device, MAX_STEPS, Readout, Rom, RunError};

use crate::{Level, TEST_SEEDS, device};

/// What a device shows before and after the last key.
type Replay = (Option<Result<Readout, RunError>>, Result<Readout, RunError>);

/// What the customer sees when a bug shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Symptom {
    /// The display shows other digits.
    WrongDigit,
    /// A light like the minus sign is on when it should be off, or the
    /// other way round.
    WrongAnnunciator,
    /// The last key changes nothing.
    NoResponse,
    /// The device stops reacting to keys.
    Freeze,
    /// The display is wrong after the clear key.
    WrongAfterClear,
}

impl Symptom {
    pub const ALL: [Self; 5] = [
        Self::WrongDigit,
        Self::WrongAnnunciator,
        Self::NoResponse,
        Self::Freeze,
        Self::WrongAfterClear,
    ];
    const CLEAR: char = 'C';

    /// What the test for `keys` shows when `buggy` runs instead of `working`.
    ///
    /// Runs like the test runner on the first test seed. Returns `None` if
    /// both show the same or `working` fails.
    pub(crate) fn shown(level: &Level, working: &Rom, buggy: &Rom, keys: &str) -> Option<Self> {
        let (working_before, working) = Self::replay(level, working, keys)?;
        let (buggy_before, buggy) = Self::replay(level, buggy, keys)?;
        let working = working.ok()?;
        if buggy
            .as_ref()
            .is_ok_and(|b| b.to_string() == working.to_string())
        {
            return None;
        }
        let working_before = working_before.and_then(Result::ok);
        let buggy_before = buggy_before.and_then(Result::ok);
        let before = working_before.as_ref().zip(buggy_before.as_ref());
        Some(Self::classify(keys, before, &working, buggy.as_ref()))
    }

    fn replay(level: &Level, rom: &Rom, keys: &str) -> Option<Replay> {
        let mut device = device(level, rom.clone()).ok()?;
        device.power_on(TEST_SEEDS[0]);
        let run =
            |device: &mut AnyDevice| device.run_until_halt(MAX_STEPS).map(|_| device.readout());
        let mut before = None;
        let mut after = run(&mut device);
        for key in keys.chars().filter(|c| !c.is_whitespace()) {
            if after.is_err() {
                break;
            }
            before = Some(after);
            after = device.press_key(key).and_then(|()| run(&mut device));
        }
        Some((before, after))
    }

    /// Names the difference between what the working firmware shows and what
    /// the mutant shows after `keys`.
    ///
    /// `before` is what the working firmware and the mutant showed before
    /// the last key.
    fn classify(
        keys: &str,
        before: Option<(&Readout, &Readout)>,
        working: &Readout,
        mutant: Result<&Readout, &RunError>,
    ) -> Self {
        let Ok(mutant) = mutant else {
            return Self::Freeze;
        };
        let unchanged = |before: &Readout, after: &Readout| before.to_string() == after.to_string();
        if let Some((working_before, mutant_before)) = before
            && unchanged(mutant_before, mutant)
            && !unchanged(working_before, working)
        {
            return Self::NoResponse;
        }
        if working.error != mutant.error
            || (working.minus != mutant.minus && working.digits == mutant.digits)
        {
            return Self::WrongAnnunciator;
        }
        if keys
            .chars()
            .any(|key| key.eq_ignore_ascii_case(&Self::CLEAR))
        {
            return Self::WrongAfterClear;
        }
        Self::WrongDigit
    }
}
