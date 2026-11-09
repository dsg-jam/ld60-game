use std::fmt;

use ld60_tick::Effect;

use crate::{MARKS, Movement};

/// Instructions a movement may run between two ticks before it counts as
/// never going to sleep.
const AWAKE_LIMIT: u32 = 1000;
/// Ticks a customer watches the hand for.
const WATCHED: u32 = 14;
/// Ticks before the customer presses the key.
const BEFORE_KEY: u32 = 5;

/// What a customer wants a watch movement to do. The flyer shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wanted {
    /// The hand moves on one mark every tick.
    Ticking,
    /// Ticking, and the key puts the hand back to the top, where it waits
    /// for the next tick.
    Resetting,
}

/// What a customer finds wrong with a watch movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchComplaint {
    /// The chip never sleeps between ticks.
    NeverWaits,
    /// The hand stood still over a tick.
    Stopped { tick: u32 },
    /// The hand moved more than one mark over a tick.
    Skipped { tick: u32, marks: u8 },
    /// After the key, the hand was not at the top.
    NotReset { mark: u8 },
}

impl Wanted {
    /// Runs a copy of `movement` from power on, as the customer would, and
    /// says what is wrong with it, if anything.
    pub fn check(self, movement: &Movement) -> Result<(), WatchComplaint> {
        let mut watch = movement.clone();
        watch.power_on();
        Self::settle(&mut watch)?;
        let mut last = watch.hand();
        for tick in 1..=WATCHED {
            watch.tick();
            Self::settle(&mut watch)?;
            let marks = (watch.hand() + MARKS - last) % MARKS;
            match marks {
                1 => {}
                0 => return Err(WatchComplaint::Stopped { tick }),
                marks => return Err(WatchComplaint::Skipped { tick, marks }),
            }
            last = watch.hand();
            if self == Self::Resetting && tick == BEFORE_KEY {
                watch.press();
                Self::settle(&mut watch)?;
                if watch.hand() != 0 {
                    return Err(WatchComplaint::NotReset { mark: watch.hand() });
                }
                last = 0;
            }
        }
        Ok(())
    }

    /// Runs until the chip sleeps.
    fn settle(watch: &mut Movement) -> Result<(), WatchComplaint> {
        for _ in 0..AWAKE_LIMIT {
            if watch.step() == Effect::Asleep {
                return Ok(());
            }
        }
        Err(WatchComplaint::NeverWaits)
    }
}

impl fmt::Display for WatchComplaint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NeverWaits => f.write_str("the chip never waits for a tick"),
            Self::Stopped { tick } => write!(f, "the hand stood still at tick {tick}"),
            Self::Skipped { tick, marks } => {
                write!(f, "the hand moved {marks} marks at tick {tick}")
            }
            Self::NotReset { mark } => write!(f, "after the key, the hand was at mark {mark}"),
        }
    }
}

impl std::error::Error for WatchComplaint {}

#[cfg(test)]
mod tests {
    use ld60_tick::{Op, Revision};

    use super::*;

    fn programmed(revision: Revision, words: &[(usize, u8)]) -> Movement {
        let mut movement = Movement::blank(revision);
        for &(addr, word) in words {
            movement
                .rom_mut()
                .program_word(addr, u16::from(word))
                .unwrap();
        }
        movement
    }

    #[test]
    fn step_and_wait_in_either_order_tick() {
        for words in [[Op::STEP, Op::WAIT], [Op::WAIT, Op::STEP]] {
            let movement = programmed(Revision::A, &[(0, words[0]), (1, words[1])]);
            assert_eq!(Wanted::Ticking.check(&movement), Ok(()));
        }
    }

    #[test]
    fn wrong_programs_have_their_complaint() {
        let blank = Movement::blank(Revision::A);
        assert_eq!(
            Wanted::Ticking.check(&blank),
            Err(WatchComplaint::NeverWaits)
        );
        let doubled = programmed(
            Revision::A,
            &[(0, Op::STEP), (1, Op::WAIT), (2, Op::STEP), (3, Op::WAIT)],
        );
        assert_eq!(Wanted::Ticking.check(&doubled), Ok(()));
        let twice = programmed(Revision::A, &[(0, Op::STEP), (1, Op::STEP), (2, Op::WAIT)]);
        assert_eq!(
            Wanted::Ticking.check(&twice),
            Err(WatchComplaint::Skipped { tick: 1, marks: 2 })
        );
        let lazy = programmed(Revision::A, &[(0, Op::STEP), (1, Op::WAIT), (2, Op::WAIT)]);
        assert!(matches!(
            Wanted::Ticking.check(&lazy),
            Err(WatchComplaint::Stopped { .. })
        ));
    }

    #[test]
    fn the_reset_needs_the_wait_first() {
        let home = (usize::from(Revision::KEY_VECTOR), Op::HOME);
        let late = programmed(Revision::B, &[(0, Op::STEP), (1, Op::WAIT), home]);
        assert_eq!(
            Wanted::Resetting.check(&late),
            Err(WatchComplaint::NotReset { mark: 1 })
        );
        let waiting = programmed(Revision::B, &[(0, Op::WAIT), (1, Op::STEP), home]);
        assert_eq!(Wanted::Resetting.check(&waiting), Ok(()));
    }
}
