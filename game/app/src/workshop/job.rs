mod bench;
mod error;

use std::error::Error;
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::{platform::hash::fixed_hash_one, prelude::*};
use ld60_devices::{DeviceKind, Rom};
use ld60_levels::{Difficulty, Generated, Level, LevelSource, Measure, Variant};
use ld60_machine::Isa;

pub(crate) use self::bench::Bench;
pub(crate) use self::error::JobError;
use super::levels::Levels;
use super::pickup::{Complaint, Payment, Pickup, TRIES};
use super::regulars::Visit;
use super::run::{Origin, Run};

pub(crate) struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Job::follow_levels.run_if(resource_exists_and_changed::<Levels>),
        );
    }
}

/// The repair job on the bench.
///
/// A job is done in two steps: flash new firmware, then stamp the ticket.
/// The customer picks the device up. The customer tests the device at pickup. If
/// it fails, they hand it back and the steps start over.
#[derive(Resource, Debug)]
pub(crate) struct Job {
    pub(crate) id: JobId,
    pub(crate) origin: Origin,
    pub(crate) level: Level,
    /// The firmware the device comes in with.
    pub(crate) buggy: String,
    /// The factory fix, as the bench measures it.
    par: Option<Measure>,
    /// The regular who brought the device, if a regular did.
    pub(crate) visit: Option<Visit>,
    /// Set once firmware was burned into the device.
    pub(crate) flashed: bool,
    /// Set once the ticket carries the shop stamp.
    pub(crate) stamped: bool,
    /// How often the customer handed the device back.
    pub(crate) returns: u32,
    /// The run as it was when the customer brought the device.
    brought: Run,
}

/// Tells one job on the bench from the next, even when both are built from
/// the same level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct JobId(u32);

/// A job came on the bench, so its device and ticket come to the counter.
#[derive(Event, Debug)]
pub(crate) struct JobOnBench;

impl JobId {
    pub(crate) fn next() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl Origin {
    /// The hand-written level the job is built from, if any.
    pub(crate) fn source(&self) -> Option<&LevelSource> {
        match self {
            Self::Lesson { source, .. } | Self::Special(source) => Some(source),
            Self::Generated { .. } => None,
        }
    }
}

impl Job {
    /// The job from `origin` for the current job of `run`. Outside the
    /// tutorial, it pays what the [`Demand`](super::run::Demand) allows.
    pub(crate) fn new(origin: Origin, run: &Run, levels: &Levels) -> Result<Self, JobError> {
        let seed = run.seed();
        let variant = match &origin {
            Origin::Lesson { source, .. } => Variant::new(source, seed)?,
            Origin::Special(source) => Variant::avoiding(source, seed, run.tired_board())?,
            &Origin::Generated { device, number } => {
                let difficulty = Difficulty::for_shop(number, device, &run.chips(), run.errata());
                let reference = levels.reference(device, difficulty.chip())?;
                Generated::avoiding(&reference, seed, &difficulty, run.tired_board())?.variant
            }
        };
        let Variant {
            mut level,
            buggy,
            fixed,
        } = variant;
        if !matches!(origin, Origin::Lesson { .. }) {
            level.reward = run.demand().pay(level.device, level.reward);
        }
        let mut visit = None;
        if let Origin::Generated { device, number } = origin {
            level.id = Self::generated_id(device, number);
            visit = run.regulars().pick(run.job(), seed);
            if let (Some(visit), Some(text)) = (visit, &mut level.text) {
                text.customer = visit.regular.id().into();
            }
        }
        let par = Measure::par(&level, &buggy, &fixed)
            .inspect_err(|err| warn!(error = err as &dyn Error, "factory fix does not measure"))
            .ok();
        Ok(Self {
            id: JobId::next(),
            origin,
            level,
            buggy,
            par,
            visit,
            flashed: false,
            stamped: false,
            returns: 0,
            brought: run.clone(),
        })
    }

    /// The current job of `run`.
    pub(crate) fn for_run(run: &Run, levels: &Levels) -> Result<Self, JobError> {
        Self::new(run.pick(levels), run, levels)
    }

    /// The instruction set the firmware of this job is written in.
    pub(crate) fn isa(&self) -> &'static dyn Isa {
        self.level.chip.isa()
    }

    /// How often the customer still tries the device.
    pub(crate) fn tries_left(&self) -> u32 {
        TRIES.saturating_sub(self.returns)
    }

    /// What the customer pays for a working device. Each return takes an
    /// equal share off the full reward, rounded to the nearest unit.
    pub(crate) fn reward(&self) -> u32 {
        (self.level.reward * self.tries_left() + TRIES / 2) / TRIES
    }

    /// Runs the customer's tests on `rom` and measures the repair if they
    /// pass.
    pub(crate) fn try_out(&self, rom: &Rom) -> Result<Measure, Complaint> {
        if let Some(complaint) = Complaint::find(&self.level, rom) {
            return Err(complaint);
        }
        Ok(Measure::new(&self.level, &self.buggy, rom)
            .inspect_err(|err| warn!(error = err as &dyn Error, "repair does not measure"))
            .unwrap_or_default())
    }

    /// Puts the current job of the run on the bench.
    pub(crate) fn start(run: Res<Run>, levels: Option<Res<Levels>>, mut commands: Commands) {
        let Some(levels) = levels else {
            return;
        };
        match Job::for_run(&run, &levels) {
            Ok(job) => {
                commands.insert_resource(job);
                commands.trigger(JobOnBench);
            }
            Err(err) => {
                error!(error = &err as &dyn Error, "next job does not load");
                commands.remove_resource::<Job>();
            }
        }
    }

    /// The customer tries the device. It either works as `measure` says or
    /// shows a complaint. The steps start over either way.
    pub(crate) fn pick_up(&mut self, tried: Result<Measure, Complaint>) -> Pickup {
        self.flashed = false;
        self.stamped = false;
        let complaint = match tried {
            Ok(measure) => {
                return Pickup::Paid(Payment {
                    fee: self.reward(),
                    returns: self.returns,
                    measure,
                    par: self.par,
                    visit: self.visit,
                });
            }
            Err(complaint) => complaint,
        };
        self.returns += 1;
        match self.tries_left() {
            0 => Pickup::Left(complaint),
            tries_left => Pickup::HandedBack {
                complaint,
                tries_left,
            },
        }
    }

    /// Puts the first job on the bench once the levels are loaded, or the
    /// [`Bench`] of a save. Rebuilds the current job when the files of its
    /// level change.
    fn follow_levels(
        levels: Res<Levels>,
        run: Res<Run>,
        job: Option<Res<Job>>,
        bench: Option<Res<Bench>>,
        mut started: Local<bool>,
        mut commands: Commands,
    ) {
        match job.as_ref().map(|job| job.origin.source()) {
            None if *started => return,
            None => {}
            Some(None) => return,
            Some(Some(source)) => match levels.find(&source.path) {
                Some(current) if current != source => {}
                _ => return,
            },
        }
        let built = match (&job, bench) {
            (Some(job), _) => Job::for_run(&job.brought, &levels),
            (None, Some(bench)) => {
                commands.remove_resource::<Bench>();
                bench.reopen(&levels)
            }
            (None, None) => Job::for_run(&run, &levels),
        };
        match built {
            Ok(job) => {
                *started = true;
                commands.insert_resource(job);
                commands.trigger(JobOnBench);
            }
            Err(err) => error!(error = &err as &dyn Error, "job does not load"),
        }
    }
}

impl Job {
    /// Stands for the customer, so each has their own drone. A regular
    /// keeps theirs.
    pub(crate) fn customer(&self) -> u64 {
        match self.visit {
            Some(visit) => fixed_hash_one(visit.regular),
            None => fixed_hash_one(&self.level.id),
        }
    }

    /// The job number of the generated job `number` for `device`, like
    /// `tally2-005`. Three digits keep it apart from the special jobs.
    fn generated_id(device: DeviceKind, number: u32) -> String {
        format!("{}-{:03}", device.id(), number + 1)
    }
}

#[cfg(test)]
impl Job {
    /// The job built from the level at `index`, as the first job of a run.
    pub(crate) fn load(levels: &Levels, index: usize) -> Option<Result<Self, JobError>> {
        let source = levels.levels().get(index)?;
        Some(Self::new(
            Origin::Special(source.clone()),
            &Run::default(),
            levels,
        ))
    }
}

#[cfg(test)]
mod tests {
    use ld60_levels::Catalog;

    use super::*;

    fn job(index: usize) -> Job {
        Job::load(&Levels::on_disk(), index).unwrap().unwrap()
    }

    /// The levels on disk with the files at `index` changed by `edit`.
    fn edited(index: usize, edit: impl FnOnce(&mut LevelSource)) -> Levels {
        let mut levels = Levels::on_disk().levels().to_vec();
        edit(&mut levels[index]);
        Levels::new(Catalog::new(Levels::order_on_disk(), levels))
    }

    fn follow(world: &mut World, levels: Levels) {
        world.insert_resource(levels);
        world.run_system_cached(Job::follow_levels).unwrap();
    }

    fn paying(reward: u32) -> Job {
        let mut job = job(0);
        job.level.reward = reward;
        job
    }

    /// The tutorial step of the job on the bench.
    fn step(world: &World) -> Option<usize> {
        match world.get_resource::<Job>()?.origin {
            Origin::Lesson { step, .. } => Some(step),
            _ => None,
        }
    }

    #[test]
    fn each_return_cuts_the_reward() {
        for (base, rewards) in [(3, [3, 2, 1]), (5, [5, 3, 2])] {
            let paid: Vec<_> = (0..TRIES)
                .map(|returns| {
                    let mut job = paying(base);
                    for _ in 0..returns {
                        job.pick_up(Err(Complaint::Dead));
                    }
                    match job.pick_up(Ok(Measure::default())) {
                        Pickup::Paid(payment) => payment.fee,
                        pickup => panic!("{pickup:?}"),
                    }
                })
                .collect();
            assert_eq!(paid, rewards, "base {base}");
        }
    }

    #[test]
    fn third_return_ends_the_job_unpaid() {
        let mut job = job(0);
        let pickups: Vec<_> = (0..TRIES)
            .map(|_| job.pick_up(Err(Complaint::Dead)))
            .collect();
        let handed_back = |tries_left| Pickup::HandedBack {
            complaint: Complaint::Dead,
            tries_left,
        };
        assert_eq!(
            pickups,
            [
                handed_back(2),
                handed_back(1),
                Pickup::Left(Complaint::Dead)
            ]
        );
        assert!(pickups[2].ends_job());
    }

    #[test]
    fn return_starts_the_steps_over() {
        let mut job = job(0);
        job.flashed = true;
        job.stamped = true;
        job.pick_up(Err(Complaint::Dead));
        assert!(!job.flashed);
        assert!(!job.stamped);
        assert_eq!(job.returns, 1);
    }

    #[test]
    fn fixing_the_first_level_starts_it() {
        let mut world = World::new();
        world.init_resource::<Run>();
        follow(&mut world, edited(0, |source| source.level.clear()));
        assert!(!world.contains_resource::<Job>());
        follow(&mut world, Levels::on_disk());
        assert_eq!(step(&world), Some(0));
    }

    #[test]
    fn generated_jobs_follow_the_tutorial() {
        let levels = Levels::on_disk();
        let mut run = Run::default().next();
        for index in 0..3 {
            let job = Job::new(run.pick(&levels), &run, &levels).unwrap();
            assert_eq!(job.level.device, DeviceKind::Tally2);
            assert!(
                index > 0 || matches!(job.origin, Origin::Generated { .. }),
                "{:?}",
                job.origin
            );
            assert!(job.level.reward >= 1);
            if let Origin::Generated { number, .. } = job.origin {
                assert_eq!(job.level.id, format!("tally2-{:03}", number + 1));
            }
            run.settle(&job, &Pickup::Paid(Payment::example(job.level.reward)));
        }
    }

    #[test]
    fn a_saved_bench_comes_back_first() {
        let levels = Levels::on_disk();
        let mut job = Job::for_run(&Run::default(), &levels).unwrap();
        job.returns = 2;
        let mut world = World::new();
        world.init_resource::<Run>();
        world.insert_resource(Bench::from(&job));
        follow(&mut world, levels);
        assert_eq!(world.resource::<Job>().returns, 2);
        assert!(!world.contains_resource::<Bench>());
    }

    #[test]
    fn editing_the_current_level_rebuilds_its_job() {
        let mut world = World::new();
        world.init_resource::<Run>();
        follow(&mut world, Levels::on_disk());
        world.resource_mut::<Job>().returns = 1;
        let edit = |source: &mut LevelSource| {
            source.level = source.level.replace("reward: 2", "reward: 9");
        };
        follow(&mut world, edited(0, edit));
        let job = world.resource::<Job>();
        assert_eq!((job.level.reward, job.returns), (9, 0));
    }
}
