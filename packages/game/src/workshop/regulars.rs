use ld60_locale::Message;
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

/// Jobs between two visits of the same regular.
const GAP: u32 = 4;
/// A regular who may come takes one in this many generated jobs.
const ODDS: u32 = 2;
/// What a regular adds on their last visit.
const GIFT: u32 = 6;
/// Keeps the picks of regulars apart from the other picks of a job.
const SEED: u64 = 0x7e6a_0f5e;
const MESSAGE: &str = "regular";

/// A local who comes back with one broken device after another and tells a
/// little more of their story each time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Regular {
    Pell,
    Ommi,
    Vosh,
    Krell,
}

/// One visit of a regular, counted from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Visit {
    pub(crate) regular: Regular,
    pub(crate) number: u32,
}

/// How far the story of each regular got in a run.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Regulars {
    seen: Vec<Seen>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Seen {
    regular: Regular,
    visits: u32,
    /// The job of the last visit.
    job: u32,
}

impl Regular {
    pub(crate) const ALL: [Self; 4] = [Self::Pell, Self::Ommi, Self::Vosh, Self::Krell];
    /// How often each regular comes in a run.
    pub(crate) const VISITS: u32 = 3;

    /// The customer id of the regular in the job text.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Pell => "pell",
            Self::Ommi => "ommi",
            Self::Vosh => "vosh",
            Self::Krell => "krell",
        }
    }

    /// The job of the run from which the regular may come.
    fn first_job(self) -> u32 {
        match self {
            Self::Pell => 3,
            Self::Ommi => 6,
            Self::Vosh => 9,
            Self::Krell => 12,
        }
    }

    fn text(self, attribute: &str, number: u32) -> Message {
        Message::new(format!("{MESSAGE}-{}", self.id())).attribute(format!("{attribute}-{number}"))
    }
}

impl Visit {
    /// What the regular tells about themselves when they bring the device.
    pub(crate) fn story(self) -> Message {
        self.regular.text("visit", self.number)
    }

    /// What the regular writes when the device works.
    pub(crate) fn thanks(self) -> Message {
        self.regular.text("thanks", self.number)
    }

    /// What the regular adds to the pay, only on their last visit.
    pub(crate) fn gift(self) -> u32 {
        if self.number == Regular::VISITS {
            GIFT
        } else {
            0
        }
    }
}

impl Regulars {
    /// The regular who brings job `job` of the run, if any. `seed` is the
    /// seed of the job.
    pub(crate) fn pick(&self, job: u32, seed: u64) -> Option<Visit> {
        let due: Vec<Visit> = Regular::ALL
            .into_iter()
            .filter(|regular| job >= regular.first_job())
            .filter_map(|regular| {
                let seen = self.seen(regular);
                let rested = seen.is_none_or(|seen| job >= seen.job + GAP);
                let visits = seen.map_or(0, |seen| seen.visits);
                (rested && visits < Regular::VISITS).then_some(Visit {
                    regular,
                    number: visits + 1,
                })
            })
            .collect();
        let mut rng = SmallRng::seed_from_u64(seed ^ SEED);
        if due.is_empty() || !rng.random_ratio(1, ODDS) {
            return None;
        }
        Some(due[rng.random_range(0..due.len())])
    }

    /// Notes that `visit` took job `job` of the run.
    pub(crate) fn record(&mut self, visit: Visit, job: u32) {
        let seen = Seen {
            regular: visit.regular,
            visits: visit.number,
            job,
        };
        match self.seen.iter_mut().find(|s| s.regular == visit.regular) {
            Some(old) => *old = seen,
            None => self.seen.push(seen),
        }
    }

    fn seen(&self, regular: Regular) -> Option<Seen> {
        self.seen.iter().copied().find(|s| s.regular == regular)
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::Locale;

    use super::*;

    /// The visits of a run of `jobs` jobs, each recorded as it comes.
    fn visits(jobs: u32) -> Vec<(u32, Visit)> {
        let mut regulars = Regulars::default();
        let mut visits = Vec::new();
        for job in 0..jobs {
            if let Some(visit) = regulars.pick(job, u64::from(job) * 7919) {
                regulars.record(visit, job);
                visits.push((job, visit));
            }
        }
        visits
    }

    #[test]
    fn each_regular_tells_the_whole_story_in_order() {
        let visits = visits(200);
        for regular in Regular::ALL {
            let numbers: Vec<_> = visits
                .iter()
                .filter(|(_, visit)| visit.regular == regular)
                .map(|(_, visit)| visit.number)
                .collect();
            assert_eq!(numbers, [1, 2, 3], "{regular:?}");
        }
    }

    #[test]
    fn regulars_rest_between_visits_and_come_late_enough() {
        let visits = visits(200);
        for (job, visit) in &visits {
            assert!(*job >= visit.regular.first_job(), "{visit:?} at {job}");
            let earlier = visits
                .iter()
                .filter(|(j, v)| j < job && v.regular == visit.regular)
                .map(|(j, _)| *j)
                .max();
            assert!(earlier.is_none_or(|e| job - e >= GAP), "{visit:?} at {job}");
        }
    }

    #[test]
    fn only_the_last_visit_brings_a_gift() {
        let gifts: Vec<_> = (1..=Regular::VISITS)
            .map(|number| {
                Visit {
                    regular: Regular::Pell,
                    number,
                }
                .gift()
            })
            .collect();
        assert_eq!(gifts, [0, 0, GIFT]);
    }

    #[test]
    fn every_visit_has_its_text() {
        let locale = Locale::on_disk();
        for regular in Regular::ALL {
            locale.assert_formats([Message::new("job-customer").attribute(regular.id())]);
            for number in 1..=Regular::VISITS {
                let visit = Visit { regular, number };
                locale.assert_formats([visit.story(), visit.thanks()]);
            }
        }
    }
}
