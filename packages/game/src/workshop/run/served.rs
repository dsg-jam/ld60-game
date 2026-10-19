use ld60_devices::DeviceKind;
use serde::{Deserialize, Serialize};

use super::Demand;

/// How many jobs the shop took on each device in a run, not counting the
/// tutorial. Jobs on a device get harder with this count, so a device whose
/// sign just went up starts with easy jobs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Served {
    /// In the order of [`DeviceKind::ALL`].
    jobs: [u32; DeviceKind::ALL.len()],
}

impl Served {
    pub(crate) fn jobs(&self, device: DeviceKind) -> u32 {
        self.jobs[Demand::index(device)]
    }

    pub(crate) fn add(&mut self, device: DeviceKind) {
        self.jobs[Demand::index(device)] += 1;
    }
}
