mod bench;
mod error;

use std::error::Error;
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::prelude::*;
use ld60_levels::{Difficulty, Generated, Level, LevelSource, Variant};
use ld60_machine::Isa;

pub(crate) use self::bench::Bench;
pub(crate) use self::error::JobError;
use super::chore::{Busy, Chore};
use super::levels::Levels;
use super::pickup::{Complaint, Pickup, TRIES};
use super::receipt::Receipt;
use super::run::{Origin, Run};
use super::ticket::JobTicket;
use crate::device::{Emulator, Handheld};
use crate::layout::counter::CounterLayout;

pub(super) const DEVICE_SPOT: Vec2 = Vec2::new(0.03, 0.3);
pub(super) const DEVICE_YAW: f32 = 0.15;
pub(super) const TICKET_SPOT: Vec2 = Vec2::new(-0.85, 0.5);
pub(super) const TICKET_YAW: f32 = 0.12;

pub(crate) struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Job::follow_levels.run_if(resource_exists_and_changed::<Levels>),
        )
        .add_observer(HandIn::pick_up);
    }
}

/// The repair job on the bench.
///
/// A job is done in three steps: flash new firmware, stamp the ticket, then
/// ring the bell to hand it in. The customer tests the device at pickup. If
/// it fails, they hand it back and the steps start over.
#[derive(Resource, Debug)]
pub(crate) struct Job {
    pub(crate) id: JobId,
    pub(crate) origin: Origin,
    pub(crate) level: Level,
    /// The firmware the device comes in with.
    pub(crate) buggy: String,
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

/// The device, ticket and notes of the current [`Job`]. They leave the desk
/// when the customer takes the device home.
#[derive(Component, Debug, Default)]
pub(crate) struct JobItem;

/// Hands in the stamped job and lets the customer test it. Once they take
/// the device home, the next job comes on the bench.
#[derive(Event, Debug)]
pub(crate) struct HandIn;

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
            mut level, buggy, ..
        } = variant;
        if !matches!(origin, Origin::Lesson { .. }) {
            level.reward = run.demand().pay(level.device, level.reward);
        }
        Ok(Self {
            id: JobId::next(),
            origin,
            level,
            buggy,
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

    /// Puts the current job of the run on the bench.
    pub(crate) fn start(run: Res<Run>, levels: Option<Res<Levels>>, mut commands: Commands) {
        let Some(levels) = levels else {
            return;
        };
        match Job::for_run(&run, &levels) {
            Ok(job) => {
                commands.insert_resource(job);
                commands.run_system_cached(Job::spawn_items);
            }
            Err(err) => {
                error!(error = &err as &dyn Error, "next job does not load");
                commands.remove_resource::<Job>();
            }
        }
    }

    /// The customer tries the device and finds `complaint`, if any. The
    /// steps start over either way.
    fn pick_up(&mut self, complaint: Option<Complaint>) -> Pickup {
        self.flashed = false;
        self.stamped = false;
        let Some(complaint) = complaint else {
            return Pickup::Paid(self.reward());
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
        items: Query<Entity, With<JobItem>>,
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
                for item in &items {
                    commands.entity(item).despawn();
                }
                commands.insert_resource(job);
                commands.run_system_cached(Job::spawn_items);
            }
            Err(err) => error!(error = &err as &dyn Error, "job does not load"),
        }
    }

    fn spawn_items(job: Option<Res<Job>>, mut commands: Commands) {
        let Some(job) = job else {
            return;
        };
        let counter = CounterLayout::new();
        commands.spawn((
            Name::new("device"),
            JobItem,
            Handheld::for_job(&job.level, &job.buggy),
            counter.place(DEVICE_SPOT, DEVICE_YAW),
        ));
        commands.spawn((
            Name::new("job ticket"),
            JobItem,
            JobTicket::for_level(&job.level),
            counter.place(TICKET_SPOT, TICKET_YAW),
        ));
    }
}

impl HandIn {
    fn pick_up(
        _: On<HandIn>,
        job: Option<ResMut<Job>>,
        items: Query<Entity, With<JobItem>>,
        emulators: Query<&Emulator, With<JobItem>>,
        receipts: Query<Entity, With<Receipt>>,
        mut run: ResMut<Run>,
        mut commands: Commands,
    ) {
        let Some(mut job) = job.filter(|job| job.stamped) else {
            return;
        };
        let complaint = match emulators.single() {
            Ok(emulator) => Complaint::find(&job.level, emulator.rom()),
            Err(_) => Some(Complaint::Dead),
        };
        let pickup = job.pick_up(complaint);
        run.settle(&job, &pickup);
        let ends_job = pickup.ends_job();
        Receipt::new(&job.level, pickup, run.till()).hand_out(receipts, &mut commands);
        let chore = if ends_job {
            Chore::HandIn
        } else {
            Chore::Return
        };
        commands.trigger(Busy(chore));
        if !ends_job {
            return;
        }
        for item in &items {
            commands.entity(item).despawn();
        }
        commands.run_system_cached(Job::start);
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
    use ld60_devices::DeviceKind;
    use ld60_levels::{Catalog, GAME_SEED, assemble, burn, initial_rom};

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

    /// A world with the job at `index` flashed and stamped, and its device
    /// running the fixed firmware if `fixed`, else the buggy one.
    fn bench(index: usize, fixed: bool) -> World {
        let mut job = job(index);
        let levels = Levels::on_disk();
        let variant = Variant::new(&levels.levels()[index], GAME_SEED).unwrap();
        let mut rom = initial_rom(&job.level, &job.buggy).unwrap();
        if fixed {
            let image = assemble(&job.level, &variant.fixed).unwrap().image;
            rom = burn(&job.level, &rom, image).unwrap();
        }
        let device = ld60_levels::device(&job.level, rom).unwrap();
        job.flashed = true;
        job.stamped = true;
        let mut world = World::new();
        world.add_observer(HandIn::pick_up);
        world.insert_resource(job);
        world.insert_resource(levels);
        world.init_resource::<Run>();
        world.spawn((JobItem, Emulator::power_on(device)));
        world
    }

    #[test]
    fn each_return_cuts_the_reward() {
        for (base, rewards) in [(3, [3, 2, 1]), (5, [5, 3, 2])] {
            let paid: Vec<_> = (0..TRIES)
                .map(|returns| {
                    let mut job = paying(base);
                    for _ in 0..returns {
                        job.pick_up(Some(Complaint::Dead));
                    }
                    job.pick_up(None)
                })
                .collect();
            assert_eq!(paid, rewards.map(Pickup::Paid), "base {base}");
        }
    }

    #[test]
    fn third_return_ends_the_job_unpaid() {
        let mut job = job(0);
        let pickups: Vec<_> = (0..TRIES)
            .map(|_| job.pick_up(Some(Complaint::Dead)))
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
        job.pick_up(Some(Complaint::Dead));
        assert!(!job.flashed);
        assert!(!job.stamped);
        assert_eq!(job.returns, 1);
    }

    #[test]
    fn ringing_twice_pays_once() {
        let mut world = bench(0, true);
        let reward = world.resource::<Job>().level.reward;
        world.trigger(HandIn);
        world.trigger(HandIn);
        world.flush();
        assert_eq!(world.resource::<Run>().till(), reward);
        assert_eq!(step(&world), Some(1));
    }

    #[test]
    fn ringing_twice_hands_back_once() {
        let mut world = bench(0, false);
        let id = world.resource::<Job>().id;
        world.trigger(HandIn);
        world.trigger(HandIn);
        world.flush();
        let job = world.resource::<Job>();
        assert_eq!((job.id, job.returns), (id, 1));
        assert_eq!(world.resource::<Run>().till(), 0);
        assert_eq!(world.query::<&JobItem>().iter(&world).count(), 1);
    }

    #[test]
    fn last_failed_try_moves_on_unpaid() {
        let last = Levels::on_disk().levels().len() - 1;
        for index in [0, last] {
            let mut world = bench(index, false);
            let id = world.resource::<Job>().id;
            let items: Vec<_> = world
                .query_filtered::<Entity, With<JobItem>>()
                .iter(&world)
                .collect();
            for _ in 0..TRIES {
                let mut job = world.resource_mut::<Job>();
                job.flashed = true;
                job.stamped = true;
                world.trigger(HandIn);
            }
            world.flush();
            assert_ne!(world.resource::<Job>().id, id, "job {index}");
            assert!(
                items.iter().all(|&item| world.get_entity(item).is_err()),
                "job {index}"
            );
            let run = world.resource::<Run>();
            assert_eq!((run.till(), run.stats().lost), (0, 1), "job {index}");
        }
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
            run.settle(&job, &Pickup::Paid(job.level.reward));
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
            source.level = source.level.replace("reward: 3", "reward: 9");
        };
        follow(&mut world, edited(0, edit));
        let job = world.resource::<Job>();
        assert_eq!((job.level.reward, job.returns), (9, 0));
        assert_eq!(world.query::<&JobItem>().iter(&world).count(), 2);
    }
}
