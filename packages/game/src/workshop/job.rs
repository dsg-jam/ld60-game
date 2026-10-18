use std::error::Error;

use bevy::prelude::*;
use ld60_levels::{CATALOG, GAME_SEED, Level, LevelError, Variant};
use ld60_machine::Isa;

use super::desk::DeskLayout;
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
        match Job::load(0) {
            Some(Ok(job)) => {
                app.insert_resource(job);
            }
            Some(Err(err)) => error!(error = &err as &dyn Error, "first job does not load"),
            None => {}
        }
        app.init_resource::<Earnings>()
            .add_systems(Startup, Job::spawn_items)
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
    /// Position in [`CATALOG`].
    pub(crate) index: usize,
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

/// The device and ticket of the current [`Job`]. They leave the desk when
/// the customer takes the device home.
#[derive(Component, Debug, Default)]
pub(crate) struct JobItem;

/// Hands in the stamped job and lets the customer test it. Once they take
/// the device home, the next job comes on the bench.
#[derive(Event, Debug)]
pub(crate) struct HandIn;

impl Job {
    /// The job at `index` in [`CATALOG`], or `None` after the last one.
    pub(crate) fn load(index: usize) -> Option<Result<Self, LevelError>> {
        let source = CATALOG.get(index)?;
        let variant = Variant::new(source, GAME_SEED);
        Some(variant.map(|Variant { level, buggy, .. }| Self {
            index,
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

    fn spawn_items(job: Option<Res<Job>>, mut commands: Commands) {
        let Some(job) = job else {
            return;
        };
        let desk = DeskLayout::new();
        commands.spawn((
            Name::new("device"),
            JobItem,
            Handheld::for_job(&job.level, &job.buggy),
            desk.place(DEVICE_SPOT, DEVICE_YAW),
        ));
        commands.spawn((
            Name::new("job ticket"),
            JobItem,
            JobTicket::for_level(&job.level),
            desk.place(TICKET_SPOT, TICKET_YAW),
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
        match Job::load(job.index + 1) {
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
        Job::load(index).unwrap().unwrap()
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
        let variant = Variant::new(&CATALOG[index], GAME_SEED).unwrap();
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
        let last = CATALOG.len() - 1;
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
}
