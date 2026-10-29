use ld60_devices::DeviceKind;
use serde::{Deserialize, Serialize};

use super::PerDevice;

/// How many jobs the shop took on each device in a run, not counting the
/// tutorial. Jobs on a device get harder with this count, so a device whose
/// sign just went up starts with easy jobs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Served {
    jobs: PerDevice,
}

impl Served {
    pub(crate) fn jobs(&self, device: DeviceKind) -> u32 {
        self.jobs[device]
    }

    pub(crate) fn add(&mut self, device: DeviceKind) {
        self.jobs[device] += 1;
    }
}
