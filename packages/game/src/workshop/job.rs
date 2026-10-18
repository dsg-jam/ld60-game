use std::error::Error;

use bevy::prelude::*;
use ld60_levels::{CATALOG, GAME_SEED, Level, LevelError, Variant};
use ld60_machine::Isa;

use super::desk::DeskLayout;
use super::pickup::{Complaint, Earnings, Pickup};
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
            .add_observer(HandIn::next_job);
    }
}

/// The repair job on the bench.
///
/// A job is done in three steps: flash new firmware, stamp the ticket, then
/// ring the bell to hand it in. The customer tests the device at pickup.
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
}

/// The device and ticket of the current [`Job`]. They leave the desk when
/// the job is handed in.
#[derive(Component, Debug, Default)]
pub(crate) struct JobItem;

/// Hands in the stamped job, lets the customer test it and puts the next
/// one on the bench.
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
        }))
    }

    /// The instruction set the firmware of this job is written in.
    pub(crate) fn isa(&self) -> &'static dyn Isa {
        self.level.chip.isa()
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
    fn next_job(
        _: On<HandIn>,
        job: Option<Res<Job>>,
        items: Query<Entity, With<JobItem>>,
        emulators: Query<&Emulator, With<JobItem>>,
        receipts: Query<Entity, With<Receipt>>,
        mut earnings: ResMut<Earnings>,
        mut commands: Commands,
    ) {
        let Some(job) = job.filter(|job| job.stamped) else {
            return;
        };
        let pickup = match emulators.single() {
            Ok(emulator) => Pickup::inspect(&job.level, emulator.rom()),
            Err(_) => Pickup::Refused(Complaint::Dead),
        };
        earnings.add(&pickup);
        Receipt::new(&job.level, pickup, *earnings).hand_out(receipts, &mut commands);
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
