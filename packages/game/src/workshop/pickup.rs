//! The customer tries the device at the counter before paying.

use std::error::Error;

use bevy::prelude::*;
use ld60_devices::Rom;
use ld60_levels::{Level, Outcome, TEST_SEEDS};

use crate::locale::Message;

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
    pub(crate) fn note(&self) -> Option<Message> {
        let (complaint, reaction) = match self {
            Self::Paid(_) => return None,
            Self::HandedBack {
                complaint,
                tries_left,
            } => (
                complaint,
                Message::new("note-tries-left").with("tries", *tries_left),
            ),
            Self::Left(complaint) => (complaint, Message::new("note-gave-up").with("tries", TRIES)),
        };
        Some(
            Message::new("note")
                .with("complaint", complaint.note())
                .with("reaction", reaction),
        )
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
    fn note(&self) -> Message {
        match self {
            Self::Dead => Message::new("note-dead"),
            Self::Wrong {
                keys,
                expected,
                shown,
            } => {
                // The readout prints broken segments as `?`.
                let id = match (keys.is_empty(), shown.contains('?')) {
                    (true, false) => "note-woke-said",
                    (true, true) => "note-woke-squiggles",
                    (false, false) => "note-pressed-said",
                    (false, true) => "note-pressed-squiggles",
                };
                Message::new(id)
                    .with("keys", keys.as_str())
                    .with("shown", shown.as_str())
                    .with("expected", expected.as_str())
            }
            Self::Froze { keys } if keys.is_empty() => Message::new("note-woke-froze"),
            Self::Froze { keys } => Message::new("note-pressed-froze").with("keys", keys.as_str()),
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
    use ld60_levels::{GAME_SEED, LevelSource, Variant, assemble, burn, initial_rom};

    use super::*;
    use crate::workshop::levels::Levels;

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
        for source in Levels::on_disk().levels() {
            let (level, rom) = fixed(source);
            assert_eq!(Complaint::find(&level, &rom), None, "{}", level.id);
        }
    }

    #[test]
    fn untouched_firmware_has_a_complaint() {
        for source in Levels::on_disk().levels() {
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
    fn notes_pick_the_sentence() {
        let wrong = Complaint::Wrong {
            keys: "12+34=".into(),
            expected: "000046".into(),
            shown: "000040".into(),
        };
        assert_eq!(
            wrong.note(),
            Message::new("note-pressed-said")
                .with("keys", "12+34=")
                .with("shown", "000040")
                .with("expected", "000046")
        );
        let garbage = Complaint::Wrong {
            keys: String::new(),
            expected: "000000".into(),
            shown: "0?00?0".into(),
        };
        assert_eq!(
            garbage.note(),
            Message::new("note-woke-squiggles")
                .with("keys", "")
                .with("shown", "0?00?0")
                .with("expected", "000000")
        );
        let froze = Complaint::Froze {
            keys: String::new(),
        };
        assert_eq!(froze.note(), Message::new("note-woke-froze"));
    }

    #[test]
    fn notes_count_the_tries_left() {
        let note = Pickup::HandedBack {
            complaint: Complaint::Dead,
            tries_left: 2,
        }
        .note();
        assert_eq!(
            note,
            Some(
                Message::new("note")
                    .with("complaint", Message::new("note-dead"))
                    .with(
                        "reaction",
                        Message::new("note-tries-left").with("tries", 2_u32)
                    )
            )
        );
        assert_eq!(Pickup::Paid(10).note(), None);
    }
}
