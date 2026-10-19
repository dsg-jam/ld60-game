use bevy::prelude::*;
use ld60_machine::Assembly;

use super::footer::Flashed;
use crate::workshop::job::{Job, JobId};

/// Where each source line of the firmware in the device sits in its ROM.
///
/// The debugging tools use it to show the lines the chip runs. It follows
/// the firmware the job came with and each flash, not the editor text.
#[derive(Resource, Debug, Default)]
pub(super) struct Listing(Option<Assembly>);

impl Listing {
    pub(super) fn assembly(&self) -> Option<&Assembly> {
        self.0.as_ref()
    }

    /// The 0-based source line of the instruction at `addr`.
    pub(super) fn line_at(&self, addr: u16) -> Option<usize> {
        self.0.as_ref()?.line_at(addr).map(|line| line - 1)
    }

    pub(super) fn follow_job(
        job: Option<Res<Job>>,
        mut loaded: Local<Option<JobId>>,
        mut listing: ResMut<Listing>,
    ) {
        let id = job.as_ref().map(|job| job.id);
        if id == *loaded {
            return;
        }
        *loaded = id;
        listing.0 = job.and_then(|job| ld60_levels::assemble(&job.level, &job.buggy).ok());
    }

    pub(super) fn follow_flash(
        flashed: On<Flashed>,
        job: Option<Res<Job>>,
        mut listing: ResMut<Listing>,
    ) {
        if let Some(job) = job {
            listing.0 = ld60_levels::assemble(&job.level, &flashed.0).ok();
        }
    }
}

#[cfg(test)]
impl Listing {
    pub(super) fn new(assembly: Assembly) -> Self {
        Self(Some(assembly))
    }
}
