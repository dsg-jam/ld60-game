//! The customer tries the device at the counter before paying.

use std::error::Error;

use bevy::prelude::*;
use ld60_devices::Rom;
use ld60_levels::{Level, Outcome, TEST_SEEDS};

/// How often a customer tries the device before they take it home broken.
pub(crate) const TRIES: u32 = 3;

/// Money taken in for repairs so far.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Earnings(pub(crate) u32);

/// What the customer did after trying the device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pickup {
    /// Everything worked and the customer paid this much.
    Paid(u32),
    /// The customer found a fault and handed the device back. They try it
    /// `tries_left` more times.
    HandedBack {
        complaint: Complaint,
        tries_left: u32,
    },
    /// The customer found a fault on the last try and took the device home
    /// without paying.
    Left(Complaint),
}

/// The first fault the customer found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Complaint {
    /// The device does not power on.
    Dead,
    /// The display showed `shown` after `keys`, not `expected`.
    Wrong {
        keys: String,
        expected: String,
        shown: String,
    },
    /// The device stopped responding during `keys`.
    Froze { keys: String },
}

impl Earnings {
    pub(crate) fn add(&mut self, pickup: &Pickup) {
        if let Pickup::Paid(amount) = pickup {
            self.0 += amount;
        }
    }
}

impl Pickup {
    /// Whether the device left the shop with the customer.
    pub(crate) fn ends_job(&self) -> bool {
        !matches!(self, Self::HandedBack { .. })
    }

    /// What the customer writes on the note if they are not happy.
    pub(crate) fn note(&self) -> Option<String> {
        let (complaint, angry) = match self {
            Self::Paid(_) => return None,
            Self::HandedBack {
                complaint,
                tries_left: 1,
            } => (
                complaint,
                "Grrr! No money for this. One more try, then I take it home.".to_owned(),
            ),
            Self::HandedBack {
                complaint,
                tries_left,
            } => (
                complaint,
                format!("Grrr! No money for this. You get {tries_left} more tries."),
            ),
            Self::Left(complaint) => (
                complaint,
                format!("{TRIES} tries is enough. I take it home broken. No money for you."),
            ),
        };
        Some(format!("{} {angry}", complaint.note()))
    }
}

impl Complaint {
    /// Runs the customer's tests against `rom` and returns the first fault,
    /// if there is one.
    pub(crate) fn find(level: &Level, rom: &Rom) -> Option<Self> {
        let report = match ld60_levels::run(level, rom, TEST_SEEDS) {
            Ok(report) => report,
            Err(err) => {
                error!(error = &err as &dyn Error, "customer tests do not run");
                return Some(Self::Dead);
            }
        };
        report
            .outcomes
            .iter()
            .find(|outcome| !outcome.passed())
            .map(Self::from)
    }

    /// What the customer says they did and saw.
    fn note(&self) -> String {
        let did = |keys: &str| {
            if keys.is_empty() {
                "I woke it up".to_owned()
            } else {
                format!("I pressed {keys}")
            }
        };
        match self {
            Self::Dead => "It does not even wake up.".into(),
            Self::Wrong {
                keys,
                expected,
                shown,
            } => {
                // The readout prints broken segments as `?`.
                let shown = if shown.contains('?') {
                    "showed angry squiggles".to_owned()
                } else {
                    format!("said {shown}")
                };
                format!(
                    "{} and the little window {shown}. It should say {expected}.",
                    did(keys)
                )
            }
            Self::Froze { keys } => format!("{} and it froze stiff.", did(keys)),
        }
    }
}

impl From<&Outcome> for Complaint {
    fn from(outcome: &Outcome) -> Self {
        let keys: String = outcome
            .case
            .keys
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        match &outcome.actual {
            Ok(shown) => Self::Wrong {
                keys,
                expected: outcome.case.display.clone(),
                shown: shown.clone(),
            },
            Err(_) => Self::Froze { keys },
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_levels::{CATALOG, GAME_SEED, LevelSource, Variant, assemble, burn, initial_rom};

    use super::*;

    /// The job as the game loads it.
    fn job(source: &LevelSource) -> Variant {
        Variant::new(source, GAME_SEED).unwrap()
    }

    fn buggy(source: &LevelSource) -> (Level, Rom) {
        let job = job(source);
        let rom = initial_rom(&job.level, &job.buggy).unwrap();
        (job.level, rom)
    }

    fn fixed(source: &LevelSource) -> (Level, Rom) {
        let job = job(source);
        let initial = initial_rom(&job.level, &job.buggy).unwrap();
        let image = assemble(&job.level, &job.fixed).unwrap().image;
        let rom = burn(&job.level, &initial, image).unwrap();
        (job.level, rom)
    }

    #[test]
    fn fixed_firmware_has_no_complaint() {
        for source in CATALOG {
            let (level, rom) = fixed(source);
            assert_eq!(Complaint::find(&level, &rom), None, "{}", level.id);
        }
    }

    #[test]
    fn untouched_firmware_has_a_complaint() {
        for source in CATALOG {
            let (level, rom) = buggy(source);
            assert!(Complaint::find(&level, &rom).is_some(), "{}", level.id);
        }
    }

    #[test]
    fn only_paid_pickups_add_earnings() {
        let mut earnings = Earnings(5);
        earnings.add(&Pickup::HandedBack {
            complaint: Complaint::Dead,
            tries_left: 2,
        });
        earnings.add(&Pickup::Left(Complaint::Dead));
        assert_eq!(earnings, Earnings(5));
        earnings.add(&Pickup::Paid(10));
        assert_eq!(earnings, Earnings(15));
    }

    #[test]
    fn notes_quote_the_keys() {
        let wrong = Complaint::Wrong {
            keys: "12+34=".into(),
            expected: "000046".into(),
            shown: "000040".into(),
        };
        assert_eq!(
            wrong.note(),
            "I pressed 12+34= and the little window said 000040. It should say 000046."
        );
        let garbage = Complaint::Wrong {
            keys: String::new(),
            expected: "000000".into(),
            shown: "0?00?0".into(),
        };
        assert_eq!(
            garbage.note(),
            "I woke it up and the little window showed angry squiggles. It should say 000000."
        );
        let froze = Complaint::Froze {
            keys: String::new(),
        };
        assert_eq!(froze.note(), "I woke it up and it froze stiff.");
    }

    #[test]
    fn notes_count_the_tries_left() {
        let note = |tries_left| {
            Pickup::HandedBack {
                complaint: Complaint::Dead,
                tries_left,
            }
            .note()
            .unwrap()
        };
        assert!(note(2).ends_with("You get 2 more tries."), "{}", note(2));
        assert!(note(1).contains("One more try"), "{}", note(1));
        assert_eq!(Pickup::Paid(10).note(), None);
    }
}
