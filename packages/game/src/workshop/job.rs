use std::error::Error;

use bevy::prelude::*;
use ld60_levels::{CATALOG, Level, LevelError, LevelSource};

use super::desk::DeskLayout;
use super::ticket::JobTicket;
use crate::device::Handheld;

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
        app.add_systems(Startup, Job::spawn_items)
            .add_observer(HandIn::next_job);
    }
}

/// The repair job on the bench.
///
/// A job is done in three steps: flash firmware that passes the customer's
/// tests, stamp the ticket, then ring the bell to hand it in.
#[derive(Resource, Debug)]
pub(crate) struct Job {
    /// Position in [`CATALOG`].
    pub(crate) index: usize,
    pub(crate) source: &'static LevelSource,
    pub(crate) level: Level,
    /// Set once flashed firmware passes all of the customer's tests.
    pub(crate) repaired: bool,
    /// Set once the repaired job's ticket carries the shop stamp.
    pub(crate) stamped: bool,
}

/// The device and ticket of the current [`Job`]. They leave the desk when
/// the job is handed in.
#[derive(Component, Debug, Default)]
pub(crate) struct JobItem;

/// Hands in the stamped job and puts the next one on the bench.
#[derive(Event, Debug)]
pub(crate) struct HandIn;

impl Job {
    /// The job at `index` in [`CATALOG`], or `None` after the last one.
    pub(crate) fn load(index: usize) -> Option<Result<Self, LevelError>> {
        let source = CATALOG.get(index)?;
        Some(source.load().map(|level| Self {
            index,
            source,
            level,
            repaired: false,
            stamped: false,
        }))
    }

    fn spawn_items(job: Option<Res<Job>>, mut commands: Commands) {
        let Some(job) = job else {
            return;
        };
        let desk = DeskLayout::new();
        commands.spawn((
            Name::new("device"),
            JobItem,
            Handheld::for_level(job.source),
            desk.place(DEVICE_SPOT, DEVICE_YAW),
        ));
        commands.spawn((
            Name::new("job ticket"),
            JobItem,
            JobTicket::for_level(job.source),
            desk.place(TICKET_SPOT, TICKET_YAW),
        ));
    }
}

impl HandIn {
    fn next_job(
        _: On<HandIn>,
        job: Option<Res<Job>>,
        items: Query<Entity, With<JobItem>>,
        mut commands: Commands,
    ) {
        let Some(job) = job.filter(|job| job.stamped) else {
            return;
        };
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
