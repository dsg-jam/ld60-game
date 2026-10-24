//! The customer tries the device at the counter before paying.

use std::error::Error;

use bevy::prelude::*;
use ld60_devices::Rom;
use ld60_levels::{Level, Measure, Outcome, TEST_SEEDS};
use ld60_locale::Message;

use super::regulars::Visit;

/// How often a customer tries the device before they take it home broken.
pub(crate) const TRIES: u32 = 3;
/// What the customer adds for a repair as small or as fast as the factory
/// fix.
pub(crate) const TIP: u32 = 1;
/// How many ways a customer has to say thanks.
const THANKS: u32 = 3;

/// What the customer did after trying the device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pickup {
    /// Everything worked and the customer paid.
    Paid(Payment),
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

/// What the customer paid for a working device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Payment {
    /// The reward, less a share for each return.
    pub(crate) fee: u32,
    /// How often the customer handed the device back first.
    pub(crate) returns: u32,
    /// The repair as the bench measured it.
    pub(crate) measure: Measure,
    /// The factory fix, if the bench could measure it.
    pub(crate) par: Option<Measure>,
    /// The regular who brought the device, if a regular did.
    pub(crate) visit: Option<Visit>,
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
        /// Some digits showed broken segments.
        broken: bool,
    },
    /// The device stopped responding during `keys`.
    Froze { keys: String },
}

impl Payment {
    /// All the customer paid.
    pub(crate) fn amount(&self) -> u32 {
        self.fee + self.tip() + self.gift()
    }

    /// What a regular adds on their last visit.
    pub(crate) fn gift(&self) -> u32 {
        self.visit.map_or(0, Visit::gift)
    }

    /// Whether the repair changed no more words than the factory fix.
    pub(crate) fn neat(&self) -> bool {
        self.par.is_some_and(|par| self.measure.neat(par))
    }

    /// Whether the repaired device runs as fast as the factory fix.
    pub(crate) fn quick(&self) -> bool {
        self.par.is_some_and(|par| self.measure.quick(par))
    }

    /// What the customer adds for a neat or quick repair.
    pub(crate) fn tip(&self) -> u32 {
        TIP * (u32::from(self.neat()) + u32::from(self.quick()))
    }

    fn note(&self, pick: u32) -> Message {
        let id = if self.returns == 0 {
            "receipt-thanks-first"
        } else {
            "receipt-thanks-late"
        };
        let thanks = match self.visit {
            Some(visit) => visit.thanks(),
            None => Message::new(id).with("pick", pick % THANKS),
        };
        let tip = match (self.neat(), self.quick()) {
            (true, true) => "receipt-tip-neat-quick",
            (true, false) => "receipt-tip-neat",
            (false, true) => "receipt-tip-quick",
            (false, false) => return thanks,
        };
        let words = self.measure.words;
        let par = self.par.map_or(words, |par| par.words);
        Message::new("receipt-note-paid")
            .with("thanks", thanks)
            .with(
                "tip",
                Message::new(tip)
                    .with("words", words)
                    .with("par", par)
                    .with("tip", self.tip()),
            )
    }
}

impl Pickup {
    /// Whether the device left the shop with the customer.
    pub(crate) fn ends_job(&self) -> bool {
        !matches!(self, Self::HandedBack { .. })
    }

    /// What the customer writes on the note. `pick` picks one of the ways
    /// they put it.
    pub(crate) fn note(&self, pick: u32) -> Message {
        let (complaint, reaction) = match self {
            Self::Paid(payment) => return payment.note(pick),
            Self::HandedBack {
                complaint,
                tries_left,
            } => (
                complaint,
                Message::new("receipt-note-tries-left").with("tries", *tries_left),
            ),
            Self::Left(complaint) => (
                complaint,
                Message::new("receipt-note-gave-up").with("tries", TRIES),
            ),
        };
        Message::new("receipt-note")
            .with("complaint", complaint.note())
            .with("reaction", reaction)
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
            Self::Dead => Message::new("receipt-note-dead"),
            Self::Wrong {
                keys,
                expected,
                shown,
                broken,
            } => {
                let id = match (keys.is_empty(), *broken) {
                    (true, false) => "receipt-note-woke-said",
                    (true, true) => "receipt-note-woke-squiggles",
                    (false, false) => "receipt-note-pressed-said",
                    (false, true) => "receipt-note-pressed-squiggles",
                };
                Message::new(id)
                    .with("keys", keys.as_str())
                    .with("shown", shown.as_str())
                    .with("expected", expected.as_str())
            }
            Self::Froze { keys } if keys.is_empty() => Message::new("receipt-note-woke-froze"),
            Self::Froze { keys } => {
                Message::new("receipt-note-pressed-froze").with("keys", keys.as_str())
            }
        }
    }
}

#[cfg(test)]
impl Payment {
    /// A first-try payment of `fee` for a repair bigger and slower than the
    /// factory fix, so without a tip.
    pub(crate) fn example(fee: u32) -> Self {
        Self {
            fee,
            returns: 0,
            measure: Measure {
                words: 3,
                cycles: 120,
            },
            par: Some(Measure {
                words: 2,
                cycles: 100,
            }),
            visit: None,
        }
    }

    /// Like [`Payment::example`], but as small and fast as the factory fix.
    pub(crate) fn tipped(fee: u32) -> Self {
        let mut payment = Self::example(fee);
        payment.measure = payment.par.unwrap();
        payment
    }
}

#[cfg(test)]
impl Complaint {
    /// One complaint for each sentence of the note.
    pub(crate) fn examples() -> Vec<Self> {
        let wrong = |keys: &str, broken| Self::Wrong {
            keys: keys.into(),
            expected: "000046".into(),
            shown: if broken { "0?0040" } else { "000040" }.into(),
            broken,
        };
        let froze = |keys: &str| Self::Froze { keys: keys.into() };
        vec![
            Self::Dead,
            wrong("", false),
            wrong("", true),
            wrong("12+34=", false),
            wrong("12+34=", true),
            froze(""),
            froze("12+"),
        ]
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
                // The readout prints broken segments as `?`.
                broken: shown.contains('?'),
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
    fn neat_and_quick_repairs_earn_a_tip() {
        let mut payment = Payment::tipped(5);
        assert_eq!(payment.amount(), 5 + 2 * TIP);
        payment.measure.words += 1;
        assert_eq!(payment.amount(), 5 + TIP);
        payment.measure.cycles += 1;
        assert_eq!(payment.amount(), 5);
        payment.par = None;
        assert!(!payment.neat() && !payment.quick());
    }

    #[test]
    fn notes_pick_the_sentence() {
        let wrong = Complaint::Wrong {
            keys: "12+34=".into(),
            expected: "000046".into(),
            shown: "000040".into(),
            broken: false,
        };
        assert_eq!(
            wrong.note(),
            Message::new("receipt-note-pressed-said")
                .with("keys", "12+34=")
                .with("shown", "000040")
                .with("expected", "000046")
        );
        let garbage = Complaint::Wrong {
            keys: String::new(),
            expected: "000000".into(),
            shown: "0?00?0".into(),
            broken: true,
        };
        assert_eq!(
            garbage.note(),
            Message::new("receipt-note-woke-squiggles")
                .with("keys", "")
                .with("shown", "0?00?0")
                .with("expected", "000000")
        );
        let froze = Complaint::Froze {
            keys: String::new(),
        };
        assert_eq!(froze.note(), Message::new("receipt-note-woke-froze"));
    }

    #[test]
    fn notes_count_the_tries_left() {
        let note = Pickup::HandedBack {
            complaint: Complaint::Dead,
            tries_left: 2,
        }
        .note(0);
        assert_eq!(
            note,
            Message::new("receipt-note")
                .with("complaint", Message::new("receipt-note-dead"))
                .with(
                    "reaction",
                    Message::new("receipt-note-tries-left").with("tries", 2_u32)
                )
        );
    }
}
