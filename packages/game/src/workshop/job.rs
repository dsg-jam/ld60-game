use std::error::Error;
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::prelude::*;
use ld60_levels::{Catalog, GAME_SEED, Level, LevelError, LevelSource, Variant};
use ld60_machine::Isa;

use super::counter::CounterLayout;
use super::levels::Levels;
use super::pickup::{Complaint, Earnings, Pickup, TRIES};
use super::receipt::Receipt;
use super::ticket::JobTicket;
use crate::device::{Emulator, Handheld};

const DEVICE_SPOT: Vec2 = Vec2::new(-0.05, 0.3);
const DEVICE_YAW: f32 = 0.15;
const TICKET_SPOT: Vec2 = Vec2::new(-0.85, 0.5);
const TICKET_YAW: f32 = 0.12;

pub(crate) struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Earnings>()
            .add_systems(
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
    /// Position in the [`Catalog`].
    pub(crate) index: usize,
    /// The files the job is built from.
    source: LevelSource,
    pub(crate) level: Level,
    /// The firmware the device comes in with.
    pub(crate) buggy: String,
    /// Set once firmware was burned into the device.
    pub(crate) flashed: bool,
    /// Set once the ticket carries the shop stamp.
    pub(crate) stamped: bool,
    /// How often the customer handed the device back.
    pub(crate) returns: u32,
}

/// Tells one job on the bench from the next, even when both are built from
/// the same level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct JobId(u32);

/// The device and ticket of the current [`Job`]. They leave the desk when
/// the customer takes the device home.
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

impl Job {
    /// The job at `index` in `catalog`, or `None` after the last one.
    pub(crate) fn load(catalog: &Catalog, index: usize) -> Option<Result<Self, LevelError>> {
        let source = catalog.levels().get(index)?;
        let variant = Variant::new(source, GAME_SEED);
        Some(variant.map(|Variant { level, buggy, .. }| Self {
            id: JobId::next(),
            index,
            source: source.clone(),
            level,
            buggy,
            flashed: false,
            stamped: false,
            returns: 0,
        }))
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

    /// Puts the first job on the bench once the levels are loaded. Rebuilds
    /// the current job when the files of its level change.
    fn follow_levels(
        levels: Res<Levels>,
        job: Option<ResMut<Job>>,
        items: Query<Entity, With<JobItem>>,
        mut started: Local<bool>,
        mut commands: Commands,
    ) {
        let index = match job {
            None if *started => return,
            None => 0,
            Some(mut job) => match levels.position(&job.source.path) {
                None => return,
                Some(index) if levels.levels()[index] == job.source => {
                    if job.index != index {
                        job.index = index;
                    }
                    return;
                }
                Some(index) => index,
            },
        };
        match Job::load(&levels, index) {
            Some(Ok(job)) => {
                *started = true;
                for item in &items {
                    commands.entity(item).despawn();
                }
                commands.insert_resource(job);
                commands.run_system_cached(Job::spawn_items);
            }
            Some(Err(err)) => error!(error = &err as &dyn Error, "job does not load"),
            None => {}
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
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn pick_up(
        _: On<HandIn>,
        job: Option<ResMut<Job>>,
        levels: Res<Levels>,
        items: Query<Entity, With<JobItem>>,
        emulators: Query<&Emulator, With<JobItem>>,
        receipts: Query<Entity, With<Receipt>>,
        mut earnings: ResMut<Earnings>,
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
        earnings.add(&pickup);
        let ends_job = pickup.ends_job();
        Receipt::new(&job.level, pickup, *earnings).hand_out(receipts, &mut commands);
        if !ends_job {
            return;
        }
        for item in &items {
            commands.entity(item).despawn();
        }
        let index = levels.position(&job.source.path).unwrap_or(job.index);
        match Job::load(&levels, index + 1) {
            Some(Ok(next)) => {
                commands.insert_resource(next);
                commands.run_system_cached(Job::spawn_items);
            }
            Some(Err(err)) => {
                error!(error = &err as &dyn Error, "next job does not load");
                commands.remove_resource::<Job>();
            }
            None => commands.remove_resource::<Job>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_levels::{assemble, burn, initial_rom};

    use super::*;

    fn job(index: usize) -> Job {
        Job::load(&Levels::on_disk(), index).unwrap().unwrap()
    }

    /// The levels on disk with the files at `index` changed by `edit`.
    fn edited(index: usize, edit: impl FnOnce(&mut LevelSource)) -> Levels {
        let mut levels = Levels::on_disk().levels().to_vec();
        edit(&mut levels[index]);
        Levels::new(Catalog::new(levels))
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
        world.init_resource::<Earnings>();
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
        assert_eq!(*world.resource::<Earnings>(), Earnings(reward));
        assert_eq!(world.resource::<Job>().index, 1);
    }

    #[test]
    fn ringing_twice_hands_back_once() {
        let mut world = bench(0, false);
        world.trigger(HandIn);
        world.trigger(HandIn);
        world.flush();
        let job = world.resource::<Job>();
        assert_eq!((job.index, job.returns), (0, 1));
        assert_eq!(*world.resource::<Earnings>(), Earnings(0));
        assert_eq!(world.query::<&JobItem>().iter(&world).count(), 1);
    }

    #[test]
    fn last_failed_try_moves_on_unpaid() {
        let last = Levels::on_disk().levels().len() - 1;
        for (index, next) in [(0, Some(1)), (last, None)] {
            let mut world = bench(index, false);
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
            assert_eq!(
                world.get_resource::<Job>().map(|job| job.index),
                next,
                "job {index}"
            );
            assert!(
                items.iter().all(|&item| world.get_entity(item).is_err()),
                "job {index}"
            );
            assert_eq!(*world.resource::<Earnings>(), Earnings(0), "job {index}");
        }
    }

    #[test]
    fn fixing_the_first_level_starts_it() {
        let mut world = World::new();
        follow(&mut world, edited(0, |source| source.level.clear()));
        assert!(!world.contains_resource::<Job>());
        follow(&mut world, Levels::on_disk());
        assert_eq!(world.resource::<Job>().index, 0);
    }

    #[test]
    fn no_job_starts_after_the_last() {
        let mut world = World::new();
        follow(&mut world, Levels::on_disk());
        world.remove_resource::<Job>();
        follow(&mut world, Levels::on_disk());
        assert!(!world.contains_resource::<Job>());
    }

    #[test]
    fn editing_the_current_level_rebuilds_its_job() {
        let mut world = World::new();
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

    #[test]
    fn reordering_the_catalog_keeps_the_next_job() {
        let mut world = bench(0, true);
        let mut levels = Levels::on_disk().levels().to_vec();
        let next = levels[2].path.clone();
        levels.swap(0, 1);
        world.insert_resource(Levels::new(Catalog::new(levels)));
        world.trigger(HandIn);
        world.flush();
        let job = world.resource::<Job>();
        assert_eq!((job.index, &job.source.path), (2, &next));
    }
}
