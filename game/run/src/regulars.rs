use std::fmt;

use ld60_levels::Regular;
use ld60_locale::Message;
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize};

/// Jobs between two visits of the same regular.
const GAP: u32 = 4;
/// A regular who may come takes one in this many generated jobs.
const ODDS: u32 = 2;
/// What a regular adds on their last visit.
const GIFT: u32 = 6;
/// Keeps the picks of regulars apart from the other picks of a job.
const SEED: u64 = 0x7e6a_0f5e;
const MESSAGE: &str = "regular";

/// One visit of a [`Regular`], counted from 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Visit {
    /// The [`Regular::id`].
    pub regular: String,
    pub number: u32,
    /// Whether the regular comes for the last time.
    pub last: bool,
}

/// How far the story of each regular got in a run.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Regulars {
    seen: Vec<Seen>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "SavedSeen")]
struct Seen {
    /// The [`Regular::id`].
    id: String,
    visits: u32,
    /// The job of the last visit.
    job: u32,
}

/// A [`Seen`] as saved. Saves from before name the regular like `Pell`
/// instead of giving the id.
#[derive(Deserialize)]
struct SavedSeen {
    #[serde(default)]
    id: String,
    #[serde(default)]
    regular: Name,
    visits: u32,
    job: u32,
}

/// A bare name, like `Pell`.
#[derive(Default)]
struct Name(String);

impl Visit {
    /// What the regular tells about themselves when they bring the device.
    pub fn story(&self) -> Message {
        self.text("visit")
    }

    /// What the regular writes when the device works.
    pub(crate) fn thanks(&self) -> Message {
        self.text("thanks")
    }

    /// What the regular adds to the pay, only on their last visit.
    pub(crate) fn gift(&self) -> u32 {
        if self.last { GIFT } else { 0 }
    }

    fn text(&self, attribute: &str) -> Message {
        Message::new(format!("{MESSAGE}-{}", self.regular))
            .attribute(format!("{attribute}-{}", self.number))
    }
}

impl Regulars {
    /// The one of `regulars` who brings job `job` of the run, if any.
    /// `seed` is the seed of the job.
    pub(crate) fn pick(&self, regulars: &[Regular], job: u32, seed: u64) -> Option<Visit> {
        let due: Vec<Visit> = regulars
            .iter()
            .filter(|regular| job >= regular.from)
            .filter_map(|regular| {
                let seen = self.seen(&regular.id);
                let rested = seen.is_none_or(|seen| job >= seen.job + GAP);
                let visits = seen.map_or(0, |seen| seen.visits);
                (rested && visits < regular.visits).then(|| Visit {
                    regular: regular.id.clone(),
                    number: visits + 1,
                    last: visits + 1 == regular.visits,
                })
            })
            .collect();
        let mut rng = SmallRng::seed_from_u64(seed ^ SEED);
        if due.is_empty() || !rng.random_ratio(1, ODDS) {
            return None;
        }
        Some(due[rng.random_range(0..due.len())].clone())
    }

    /// Notes that `visit` took job `job` of the run. A regular who was not
    /// `served` still rests before coming back, but comes back with the
    /// same visit, so their story goes on from where it was.
    pub(crate) fn record(&mut self, visit: &Visit, job: u32, served: bool) {
        let seen = Seen {
            id: visit.regular.clone(),
            visits: if served {
                visit.number
            } else {
                visit.number - 1
            },
            job,
        };
        match self.seen.iter_mut().find(|s| s.id == visit.regular) {
            Some(old) => *old = seen,
            None => self.seen.push(seen),
        }
    }

    fn seen(&self, regular: &str) -> Option<&Seen> {
        self.seen.iter().find(|s| s.id == regular)
    }
}

impl From<SavedSeen> for Seen {
    fn from(saved: SavedSeen) -> Self {
        Self {
            id: if saved.id.is_empty() {
                saved.regular.0.to_lowercase()
            } else {
                saved.id
            },
            visits: saved.visits,
            job: saved.job,
        }
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_identifier(NameVisitor)
    }
}

struct NameVisitor;

impl Visitor<'_> for NameVisitor {
    type Value = Name;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("the name of a regular")
    }

    fn visit_str<E>(self, name: &str) -> Result<Name, E> {
        Ok(Name(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::Locale;

    use super::*;
    use crate::levels::Levels;

    fn roster() -> Vec<Regular> {
        Levels::on_disk().regulars().to_vec()
    }

    /// The visits of a run of `jobs` jobs, each recorded as it comes.
    fn visits(jobs: u32) -> Vec<(u32, Visit)> {
        let roster = roster();
        let mut regulars = Regulars::default();
        let mut visits = Vec::new();
        for job in 0..jobs {
            if let Some(visit) = regulars.pick(&roster, job, u64::from(job) * 7919) {
                regulars.record(&visit, job, true);
                visits.push((job, visit));
            }
        }
        visits
    }

    #[test]
    fn each_regular_tells_the_whole_story_in_order() {
        let visits = visits(200);
        for regular in roster() {
            let told: Vec<_> = visits
                .iter()
                .filter(|(_, visit)| visit.regular == regular.id)
                .map(|(_, visit)| (visit.number, visit.last))
                .collect();
            let story: Vec<_> = (1..=regular.visits)
                .map(|number| (number, number == regular.visits))
                .collect();
            assert_eq!(told, story, "{}", regular.id);
        }
    }

    #[test]
    fn regulars_rest_between_visits_and_come_late_enough() {
        let roster = roster();
        let visits = visits(200);
        for (job, visit) in &visits {
            let from = roster.iter().find(|r| r.id == visit.regular).unwrap().from;
            assert!(*job >= from, "{visit:?} at {job}");
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
        let gifts: Vec<_> = visits(200)
            .into_iter()
            .filter(|(_, visit)| visit.regular == "pell")
            .map(|(_, visit)| visit.gift())
            .collect();
        assert_eq!(gifts, [0, 0, GIFT]);
    }

    #[test]
    fn every_visit_has_its_text() {
        let locale = Locale::on_disk();
        for regular in roster() {
            locale.assert_formats([Message::new("job-customer").attribute(regular.id.clone())]);
            for number in 1..=regular.visits {
                let visit = Visit {
                    regular: regular.id.clone(),
                    number,
                    last: number == regular.visits,
                };
                locale.assert_formats([visit.story(), visit.thanks()]);
            }
        }
    }

    #[test]
    fn a_regular_left_unserved_comes_back_with_the_same_visit() {
        let roster = roster();
        let mut regulars = Regulars::default();
        let job = (0..100)
            .find(|&job| regulars.pick(&roster, job, 1).is_some())
            .unwrap();
        let visit = regulars.pick(&roster, job, 1).unwrap();
        regulars.record(&visit, job, false);
        let again = (job + 1..job + 100)
            .find_map(|later| {
                regulars
                    .pick(&roster, later, 1)
                    .filter(|next| next.regular == visit.regular)
            })
            .unwrap();
        assert_eq!(again.number, visit.number);
    }

    #[test]
    fn saves_from_before_name_regulars_like_pell() {
        let old = "(seen: [(regular: Pell, visits: 2, job: 9)])";
        let regulars: Regulars = ron::from_str(old).unwrap();
        let roster = roster();
        let visit = (10..60)
            .find_map(|job| {
                regulars
                    .pick(&roster, job, u64::from(job))
                    .filter(|visit| visit.regular == "pell")
            })
            .unwrap();
        assert_eq!((visit.number, visit.last), (3, true));
        let saved = ron::to_string(&regulars).unwrap();
        assert_eq!(ron::from_str::<Regulars>(&saved).unwrap(), regulars);
    }
}
