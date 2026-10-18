use std::fmt;

use ld60_devices::{ChipSpec, DeviceKind};

use crate::backend::{Backend, CalcoBackend};

/// The device and chip a firmware is compiled for.
pub struct Target {
    pub(crate) backend: Box<dyn Backend>,
    pub(crate) chip: ChipSpec,
    pub(crate) device: DeviceKind,
}

impl Target {
    pub fn new(chip: ChipSpec, device: DeviceKind) -> Self {
        let backend = match chip {
            ChipSpec::Calco(info) => Box::new(CalcoBackend::new(info.revision)),
        };
        Self {
            backend,
            chip,
            device,
        }
    }

    /// The name used after `for` in a firmware declaration.
    pub(crate) fn device_name(&self) -> String {
        format!("{:?}", self.device).to_ascii_lowercase()
    }

    /// The code the firmware reads from the key port for `key`.
    pub(crate) fn key_code(&self, key: char) -> Option<u8> {
        self.device
            .key_codes()
            .iter()
            .find(|&&(k, _)| k == key)
            .map(|&(_, code)| code)
    }
}

impl fmt::Debug for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Target")
            .field("chip", &self.chip)
            .field("device", &self.device)
            .finish_non_exhaustive()
    }
}
