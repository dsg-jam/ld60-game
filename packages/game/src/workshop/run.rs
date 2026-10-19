mod demand;

use bevy::prelude::*;
use ld60_devices::DeviceKind;
use ld60_levels::{Catalog, GAME_SEED, LevelSource};
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

pub(crate) use self::demand::Demand;
use super::clock::Clock;
use super::job::{Job, JobItem};
use super::pickup::Pickup;
use super::receipt::Receipt;
use super::rent::RentNote;
use crate::player::focus::{Focus, Focused};

/// What the landlord takes the first time the planet sets.
const RENT: u32 = 10;
/// How much more the landlord takes each time after that.
const RENT_RISE: u32 = 6;
/// A special job that can come takes one in this many jobs.
const SPECIAL_ODDS: u32 = 3;
/// Keeps the seeds of one run away from those of the next.
const RUN_SEEDS: u64 = 1 << 32;

/// What is left on the counter when the shop closes.
type Leftovers = Or<(With<JobItem>, With<Receipt>, With<RentNote>)>;

pub(crate) struct RunPlugin;

impl Plugin for RunPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Run>().add_observer(Run::restart);
    }
}

/// One run of the shop, from opening until the landlord closes it.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Run {
    /// Counts the runs since the game started, from 0.
    number: u32,
    /// The current job, counted from 0.
    job: u32,
    /// Whether the run starts with the tutorial.
    tutorial: bool,
    /// Money in the till.
    till: u32,
    /// The devices with a sign in the shop window. Locals only bring these.
    signs: Vec<DeviceKind>,
    demand: Demand,
    /// The level folders of the special jobs done in this run.
    specials: Vec<String>,
    stats: Stats,
}

/// What the shop did in a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stats {
    /// Jobs the customer paid for.
    pub(crate) fixed: u32,
    /// Jobs the customer took home broken.
    pub(crate) lost: u32,
    /// All money taken in.
    pub(crate) earned: u32,
    /// All rent paid.
    pub(crate) rent: u32,
    /// How often the planet set with the rent paid.
    pub(crate) cycles: u32,
}

/// Where a job comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    /// Step `step` of the tutorial, counted from 0.
    Lesson { step: usize, source: LevelSource },
    /// A hand-written job between the generated ones.
    Special(LevelSource),
    /// Generated job `number` on `device`, counted from 0 after the tutorial.
    Generated { device: DeviceKind, number: u32 },
}

/// The landlord's visit when the planet sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rent {
    /// The landlord took `amount` from the till and comes for `next` the
    /// next time.
    Paid { amount: u32, next: u32 },
    /// The till held only `till` of `amount`, so the shop closes.
    Short { amount: u32, till: u32 },
}

/// Starts a new run after the shop closed.
#[derive(Event, Debug)]
pub(crate) struct NewRun;

impl Default for Run {
    fn default() -> Self {
        Self {
            number: 0,
            job: 0,
            tutorial: true,
            till: 0,
            signs: vec![DeviceKind::Tally2],
            demand: Demand::default(),
            specials: Vec::new(),
            stats: Stats::default(),
        }
    }
}

impl Run {
    /// The run after this one. The player knows the shop by then, so it
    /// skips the tutorial.
    pub(crate) fn next(&self) -> Self {
        Self {
            number: self.number + 1,
            tutorial: false,
            ..Self::default()
        }
    }

    pub(crate) fn till(&self) -> u32 {
        self.till
    }

    pub(crate) fn stats(&self) -> Stats {
        self.stats
    }

    pub(crate) fn demand(&self) -> &Demand {
        &self.demand
    }

    /// The seed of the current job.
    pub(crate) fn seed(&self) -> u64 {
        GAME_SEED + RUN_SEEDS * u64::from(self.number) + u64::from(self.job)
    }

    /// Where the current job comes from.
    ///
    /// The tutorial comes first. After it, a special job can come once its
    /// sign hangs in the window and the run has reached [`Special::from`].
    /// Otherwise the job is generated for one of the signs, picked by
    /// [`Demand`].
    ///
    /// [`Special::from`]: ld60_levels::Special::from
    pub(crate) fn pick(&self, catalog: &Catalog) -> Origin {
        let tutorial = if self.tutorial {
            catalog.tutorial()
        } else {
            &[]
        };
        let job = self.job as usize;
        if let Some(source) = tutorial.get(job) {
            return Origin::Lesson {
                step: job,
                source: source.clone(),
            };
        }
        let number = (job - tutorial.len()) as u32;
        let mut rng = SmallRng::seed_from_u64(self.seed());
        let special = catalog.specials().find(|(source, from)| {
            number >= *from
                && !self.specials.contains(&source.path)
                && source
                    .load()
                    .is_ok_and(|level| self.signs.contains(&level.device))
        });
        if let Some((source, _)) = special
            && rng.random_ratio(1, SPECIAL_ODDS)
        {
            return Origin::Special(source.clone());
        }
        let device = self
            .demand
            .pick(&self.signs, &mut rng)
            .unwrap_or(DeviceKind::Tally2);
        Origin::Generated { device, number }
    }

    /// Takes in what the customer paid for `job`. Once the device leaves
    /// the shop, the run moves on to the next job.
    pub(crate) fn settle(&mut self, job: &Job, pickup: &Pickup) {
        if let Pickup::Paid(amount) = pickup {
            self.till += amount;
            self.stats.earned += amount;
        }
        if !pickup.ends_job() {
            return;
        }
        self.job += 1;
        match pickup {
            Pickup::Paid(_) => {
                self.stats.fixed += 1;
                if !matches!(job.origin, Origin::Lesson { .. }) {
                    self.demand.fixed(job.level.device);
                }
            }
            Pickup::HandedBack { .. } | Pickup::Left(_) => self.stats.lost += 1,
        }
        if let Origin::Special(source) = &job.origin {
            self.specials.push(source.path.clone());
        }
    }

    /// What the landlord takes the next time the planet sets.
    pub(crate) fn rent(&self) -> u32 {
        RENT + RENT_RISE * self.stats.cycles
    }

    /// The landlord takes the rent from the till, if it holds enough.
    pub(crate) fn pay_rent(&mut self) -> Rent {
        let amount = self.rent();
        if self.till < amount {
            return Rent::Short {
                amount,
                till: self.till,
            };
        }
        self.till -= amount;
        self.stats.rent += amount;
        self.stats.cycles += 1;
        self.demand.recover();
        Rent::Paid {
            amount,
            next: self.rent(),
        }
    }

    /// Clears the counter and opens the shop again with an empty till.
    fn restart(
        _: On<NewRun>,
        mut run: ResMut<Run>,
        mut clock: ResMut<Clock>,
        mut focused: ResMut<Focused>,
        mut focus: ResMut<NextState<Focus>>,
        leftovers: Query<Entity, Leftovers>,
        mut commands: Commands,
    ) {
        *run = run.next();
        *clock = Clock::default();
        focused.leave(&mut focus);
        for entity in &leftovers {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<Job>();
        commands.run_system_cached(Job::start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::focus::{FocusOn, FocusTarget, View};
    use crate::workshop::levels::Levels;
    use crate::workshop::pickup::Complaint;

    fn job(run: &Run) -> Job {
        Job::new(run.pick(&Levels::on_disk()), run, &Levels::on_disk()).unwrap()
    }

    #[test]
    fn the_tutorial_comes_first() {
        let levels = Levels::on_disk();
        let mut run = Run::default();
        for (step, source) in levels.tutorial().iter().enumerate() {
            assert_eq!(
                run.pick(&levels),
                Origin::Lesson {
                    step,
                    source: source.clone()
                }
            );
            run.job += 1;
        }
        assert_eq!(
            run.pick(&levels),
            Origin::Generated {
                device: DeviceKind::Tally2,
                number: 0
            }
        );
    }

    #[test]
    fn a_new_run_skips_the_tutorial() {
        let run = Run {
            job: 5,
            till: 30,
            ..Run::default()
        };
        let next = run.next();
        assert_eq!((next.job, next.till), (0, 0));
        assert_ne!(next.seed(), Run::default().seed());
        assert_eq!(
            next.pick(&Levels::on_disk()),
            Origin::Generated {
                device: DeviceKind::Tally2,
                number: 0
            }
        );
    }

    #[test]
    fn specials_need_their_sign() {
        let levels = Levels::on_disk();
        let mut run = Run::default().next();
        let picks = |run: &mut Run| {
            (0..40)
                .map(|job| {
                    run.job = job;
                    run.pick(&levels)
                })
                .collect::<Vec<_>>()
        };
        assert!(
            picks(&mut run)
                .iter()
                .all(|origin| matches!(origin, Origin::Generated { .. }))
        );
        run.signs.push(DeviceKind::Summa6);
        let specials: Vec<_> = picks(&mut run)
            .into_iter()
            .enumerate()
            .filter_map(|(job, origin)| match origin {
                Origin::Special(source) => Some((job as u32, source.path)),
                _ => None,
            })
            .collect();
        assert!(!specials.is_empty());
        for (job, path) in specials {
            let (_, from) = levels
                .specials()
                .find(|(source, _)| source.path == path)
                .unwrap();
            assert!(job >= from, "{path} at job {job}");
        }
    }

    #[test]
    fn a_special_comes_once() {
        let levels = Levels::on_disk();
        let mut run = Run::default().next();
        run.signs = vec![DeviceKind::Summa6];
        run.job = 100;
        let mut seen = Vec::new();
        while run.job < 300 && seen.len() < levels.specials().count() {
            let Origin::Special(source) = run.pick(&levels) else {
                run.job += 1;
                continue;
            };
            assert!(!seen.contains(&source.path), "{}", source.path);
            seen.push(source.path.clone());
            run.settle(&job(&run), &Pickup::Paid(1));
        }
        assert_eq!(seen.len(), levels.specials().count());
    }

    #[test]
    fn settling_counts_the_job() {
        let mut run = Run::default().next();
        let job = job(&run);
        let handed_back = Pickup::HandedBack {
            complaint: Complaint::Dead,
            tries_left: 2,
        };
        run.settle(&job, &handed_back);
        assert_eq!((run.job, run.till), (0, 0));
        run.settle(&job, &Pickup::Paid(4));
        assert_eq!((run.job, run.till), (1, 4));
        run.settle(&job, &Pickup::Left(Complaint::Dead));
        assert_eq!(
            run.stats,
            Stats {
                fixed: 1,
                lost: 1,
                earned: 4,
                ..Stats::default()
            }
        );
        assert!(run.demand.percent(DeviceKind::Tally2) < 100);
    }

    #[test]
    fn the_tutorial_leaves_demand_alone() {
        let mut run = Run::default();
        run.settle(&job(&run), &Pickup::Paid(4));
        assert_eq!((run.job, run.stats.fixed), (1, 1));
        assert_eq!(run.demand, Demand::default());
    }

    #[test]
    fn a_new_run_leaves_the_focus() {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.init_resource::<Clock>();
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.add_observer(Run::restart);
        world.add_observer(FocusOn::apply);
        let note = world
            .spawn(FocusTarget {
                prompt: "click-rent",
                focus: Focus::Paper,
                view: View::World(Transform::IDENTITY),
            })
            .id();
        world.trigger(FocusOn { entity: note });
        assert_eq!(world.resource::<Focused>().get(), Some(note));
        world.trigger(NewRun);
        world.flush();
        assert_eq!(world.resource::<Focused>().get(), None);
        assert!(matches!(
            *world.resource::<NextState<Focus>>(),
            NextState::Pending(Focus::Desk)
        ));
    }

    #[test]
    fn worn_demand_lowers_the_reward() {
        let levels = Levels::on_disk();
        let run = Run::default().next();
        let mut worn = run.clone();
        for _ in 0..10 {
            worn.demand.fixed(DeviceKind::Tally2);
        }
        let origin = run.pick(&levels);
        let fresh = Job::new(origin.clone(), &run, &levels).unwrap();
        let tired = Job::new(origin, &worn, &levels).unwrap();
        assert!(tired.level.reward < fresh.level.reward);
    }

    #[test]
    fn tutorial_jobs_pay_in_full() {
        let levels = Levels::on_disk();
        let mut run = Run::default();
        for _ in 0..10 {
            run.demand.fixed(DeviceKind::Tally2);
        }
        let job = job(&run);
        let full = levels.tutorial()[0].load().unwrap().reward;
        assert_eq!(job.level.reward, full);
    }

    #[test]
    fn rent_grows_each_cycle() {
        let mut run = Run {
            till: 100,
            ..Run::default()
        };
        assert_eq!(
            run.pay_rent(),
            Rent::Paid {
                amount: RENT,
                next: RENT + RENT_RISE
            }
        );
        assert_eq!(
            run.pay_rent(),
            Rent::Paid {
                amount: RENT + RENT_RISE,
                next: RENT + 2 * RENT_RISE
            }
        );
        assert_eq!(run.till, 100 - 2 * RENT - RENT_RISE);
        assert_eq!(
            (run.stats.cycles, run.stats.rent),
            (2, 2 * RENT + RENT_RISE)
        );
    }

    #[test]
    fn short_rent_leaves_the_till() {
        let mut run = Run {
            till: RENT - 1,
            ..Run::default()
        };
        assert_eq!(
            run.pay_rent(),
            Rent::Short {
                amount: RENT,
                till: RENT - 1
            }
        );
        assert_eq!((run.till, run.stats.cycles), (RENT - 1, 0));
    }
}
