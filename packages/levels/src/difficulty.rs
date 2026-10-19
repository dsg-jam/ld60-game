use std::ops::RangeInclusive;

use ld60_calco::{ChipInfo, DateCode, Errata, Revision};
use ld60_devices::{ChipSpec, DeviceKind};

use crate::variant::rng::Rng;
use crate::{Operator, Reference};

/// The most keys the test that shows a bug can have.
pub(crate) const MAX_SYMPTOM_KEYS: usize = 6;
/// Chips made after the `DS` errata was fixed.
const LATE: DateCode = DateCode::new(76, 30);
/// Rev B chips with the `DS` errata.
const EARLY: DateCode = DateCode::new(76, 12);

/// How hard a generated job is.
///
/// Jobs get harder with the job number: more bugs, bugs of more kinds,
/// older chips, harmless faults on the board and longer key sequences
/// before a bug shows.
///
/// ```
/// use ld60_devices::DeviceKind;
/// use ld60_levels::Difficulty;
///
/// let first = Difficulty::for_job(0, DeviceKind::Summa6);
/// let later = Difficulty::for_job(20, DeviceKind::Summa6);
/// assert!(first.bugs() < later.bugs());
/// assert!(first.reward(1) < later.reward(1));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difficulty {
    device: DeviceKind,
    job: u32,
    bugs: usize,
    weights: [u32; Operator::ALL.len()],
    chip: ChipSpec,
    fault_chance: u32,
    symptom_keys: RangeInclusive<usize>,
}

impl Difficulty {
    /// The difficulty of job `job` on `device`.
    pub fn for_job(job: u32, device: DeviceKind) -> Self {
        Self::for_shop(job, device, &Revision::ALL)
    }

    /// The difficulty of job `job` on `device` in a shop with the errata
    /// pages of the `errata` revisions. A chip with a known bug only comes
    /// once the shop has the errata pages that explain it.
    ///
    /// ```
    /// use ld60_calco::Errata;
    /// use ld60_devices::{ChipSpec, DeviceKind};
    /// use ld60_levels::Difficulty;
    ///
    /// for job in 0..40 {
    ///     let difficulty = Difficulty::for_shop(job, DeviceKind::Summa6, &[]);
    ///     let ChipSpec::Calco(chip) = difficulty.chip();
    ///     assert_eq!(Errata::for_chip(chip), Errata::default());
    /// }
    /// ```
    pub fn for_shop(job: u32, device: DeviceKind, errata: &[Revision]) -> Self {
        let n = job as usize;
        Self {
            device,
            job,
            bugs: (1 + n / 6).min(3),
            weights: [
                4,
                4,
                job.min(4),
                (job / 2).min(4),
                (job.saturating_sub(4) / 2).min(4),
            ],
            chip: Self::pick_chip(job, device, errata),
            fault_chance: (job.saturating_sub(4) * 5).min(30),
            symptom_keys: (n / 8).min(2)..=(2 + n / 2).min(MAX_SYMPTOM_KEYS),
        }
    }

    /// How many bugs to add, from 1 to 3.
    pub fn bugs(&self) -> usize {
        self.bugs
    }

    /// How often each of [`Operator::ALL`] is picked, relative to the others.
    pub fn weights(&self) -> &[u32; Operator::ALL.len()] {
        &self.weights
    }

    /// The chip the job runs on.
    pub fn chip(&self) -> ChipSpec {
        self.chip
    }

    /// The chance in percent of a stuck RAM bit that the working firmware
    /// does not notice.
    pub fn fault_chance(&self) -> u32 {
        self.fault_chance
    }

    /// How many keys the shortest test that shows the first bug may have.
    /// The other bugs may show with fewer.
    pub fn symptom_keys(&self) -> &RangeInclusive<usize> {
        &self.symptom_keys
    }

    /// What the customer pays for a job with `bugs` bugs.
    ///
    /// Bigger devices pay more per bug. Later jobs pay a bonus.
    pub fn reward(&self, bugs: usize) -> u32 {
        let base = match self.device {
            DeviceKind::Tally2 => 4,
            DeviceKind::Summa6 => 10,
        };
        base * bugs as u32 + base * self.job / 10
    }

    /// Picks an operator by the [`Difficulty::weights`].
    pub(crate) fn operator(&self, rng: &mut Rng) -> Operator {
        let total: u32 = self.weights.iter().sum();
        let mut pick = rng.below(total as usize) as u32;
        for (operator, &weight) in Operator::ALL.iter().zip(&self.weights) {
            if pick < weight {
                return *operator;
            }
            pick -= weight;
        }
        unreachable!("the pick is below the total weight")
    }

    /// The first jobs get the newest chip. Then every revision shows up, and
    /// later the Rev B chips with the `DS` errata, if the shop has its
    /// errata pages.
    fn pick_chip(job: u32, device: DeviceKind, errata: &[Revision]) -> ChipSpec {
        let revisions = Reference::revisions(device);
        let mut chips: Vec<ChipInfo> = match job {
            0..3 => revisions.last().into_iter().copied().collect(),
            _ => revisions.to_vec(),
        }
        .into_iter()
        .map(|revision| ChipInfo {
            revision,
            date_code: LATE,
        })
        .collect();
        let early = ChipInfo {
            revision: Revision::B,
            date_code: EARLY,
        };
        let explained = Errata::for_chip(early)
            .iter()
            .all(|erratum| errata.contains(&erratum.revision()));
        if job >= 8 && revisions.contains(&Revision::B) && explained {
            chips.push(early);
        }
        let pick = Rng::new(u64::from(job), "chip").below(chips.len());
        ChipSpec::Calco(chips[pick])
    }
}
