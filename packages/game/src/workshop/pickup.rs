//! The customer tries the device at the counter before paying.

use std::error::Error;

use bevy::prelude::*;
use ld60_devices::Rom;
use ld60_levels::{Level, Outcome, TEST_SEEDS};

/// Money taken in for repairs so far.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Earnings(pub(crate) u32);

/// What the customer did after trying the device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pickup {
    /// Everything worked and the customer paid this much.
    Paid(u32),
    /// The customer found a fault and paid nothing.
    Refused(Complaint),
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
    /// Runs the customer's tests against `rom`.
    pub(crate) fn inspect(level: &Level, rom: &Rom) -> Self {
        let report = match ld60_levels::run(level, rom, TEST_SEEDS) {
            Ok(report) => report,
            Err(err) => {
                error!(error = &err as &dyn Error, "customer tests do not run");
                return Self::Refused(Complaint::Dead);
            }
        };
        match report.outcomes.iter().find(|outcome| !outcome.passed()) {
            Some(outcome) => Self::Refused(Complaint::from(outcome)),
            None => Self::Paid(level.reward),
        }
    }
}

impl Complaint {
    /// What the customer writes on the note.
    pub(crate) fn note(&self) -> String {
        let did = |keys: &str| {
            if keys.is_empty() {
                "I switched it on".to_owned()
            } else {
                format!("I typed {keys}")
            }
        };
        match self {
            Self::Dead => "It does not even switch on.".into(),
            Self::Wrong {
                keys,
                expected,
                shown,
            } => {
                // The readout prints broken segments as `?`.
                let shown = if shown.contains('?') {
                    "strange symbols"
                } else {
                    shown
                };
                format!(
                    "{} and it showed {shown}. It should say {expected}.",
                    did(keys)
                )
            }
            Self::Froze { keys } => format!("{} and it froze.", did(keys)),
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
    use ld60_levels::{CATALOG, LevelSource, assemble, burn, initial_rom};

    use super::*;

    fn buggy(source: &LevelSource) -> (Level, Rom) {
        let level = source.load().unwrap();
        let rom = initial_rom(&level, source.buggy).unwrap();
        (level, rom)
    }

    fn fixed(source: &LevelSource) -> (Level, Rom) {
        let (level, initial) = buggy(source);
        let image = assemble(&level, source.fixed).unwrap().image;
        let rom = burn(&level, &initial, image).unwrap();
        (level, rom)
    }

    #[test]
    fn fixed_firmware_is_paid() {
        for source in CATALOG {
            let (level, rom) = fixed(source);
            assert_eq!(
                Pickup::inspect(&level, &rom),
                Pickup::Paid(level.reward),
                "{}",
                level.id
            );
        }
    }

    #[test]
    fn untouched_firmware_is_refused() {
        for source in CATALOG {
            let (level, rom) = buggy(source);
            assert!(
                matches!(Pickup::inspect(&level, &rom), Pickup::Refused(_)),
                "{}",
                level.id
            );
        }
    }

    #[test]
    fn only_paid_pickups_add_earnings() {
        let mut earnings = Earnings(5);
        earnings.add(&Pickup::Refused(Complaint::Dead));
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
            "I typed 12+34= and it showed 000040. It should say 000046."
        );
        let garbage = Complaint::Wrong {
            keys: String::new(),
            expected: "000000".into(),
            shown: "0?00?0".into(),
        };
        assert_eq!(
            garbage.note(),
            "I switched it on and it showed strange symbols. It should say 000000."
        );
        let froze = Complaint::Froze {
            keys: String::new(),
        };
        assert_eq!(froze.note(), "I switched it on and it froze.");
    }
}
