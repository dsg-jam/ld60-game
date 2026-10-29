use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Job, JobError};
use crate::Run;
use crate::levels::Levels;

/// The job on the bench as a save keeps it.
///
/// The job is built again from the run as it was when the customer brought
/// the device, so purchases and rent since then do not change it.
#[derive(Resource, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bench {
    brought: Run,
    /// How often the customer handed the device back.
    returns: u32,
}

impl Bench {
    /// Whether this is the current job of `run`.
    pub fn belongs_to(&self, run: &Run) -> bool {
        self.brought.seed() == run.seed()
    }

    pub fn reopen(&self, levels: &Levels) -> Result<Job, JobError> {
        let mut job = Job::for_run(&self.brought, levels)?;
        job.returns = self.returns;
        Ok(job)
    }
}

impl From<&Job> for Bench {
    fn from(job: &Job) -> Self {
        Self {
            brought: job.brought.clone(),
            returns: job.returns,
        }
    }
}
