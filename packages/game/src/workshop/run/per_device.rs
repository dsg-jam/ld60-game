use std::fmt;
use std::ops::{Index, IndexMut};

use ld60_devices::DeviceKind;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A count for each device, in the order of [`DeviceKind::ALL`].
///
/// A save from before a device existed holds fewer counts. The missing ones
/// load as 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PerDevice([u32; DeviceKind::ALL.len()]);

impl PerDevice {
    fn index(device: DeviceKind) -> usize {
        DeviceKind::ALL
            .iter()
            .position(|&kind| kind == device)
            .unwrap_or_default()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut u32> {
        self.0.iter_mut()
    }
}

impl Index<DeviceKind> for PerDevice {
    type Output = u32;

    fn index(&self, device: DeviceKind) -> &u32 {
        &self.0[Self::index(device)]
    }
}

impl IndexMut<DeviceKind> for PerDevice {
    fn index_mut(&mut self, device: DeviceKind) -> &mut u32 {
        &mut self.0[Self::index(device)]
    }
}

impl Serialize for PerDevice {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PerDevice {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Arrays are tuples in a save, so any length has to read as one.
        deserializer.deserialize_tuple(usize::MAX, PerDevice::default())
    }
}

impl<'de> Visitor<'de> for PerDevice {
    type Value = Self;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a count for each device")
    }

    fn visit_seq<A: SeqAccess<'de>>(mut self, mut seq: A) -> Result<Self, A::Error> {
        let mut slots = self.0.iter_mut();
        while let Some(count) = seq.next_element::<u32>()? {
            if let Some(slot) = slots.next() {
                *slot = count;
            }
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_with_fewer_devices_loads() {
        let loaded: PerDevice = ron::from_str("(3, 1, 4)").unwrap();
        assert_eq!(loaded[DeviceKind::Tally2], 3);
        assert_eq!(loaded[DeviceKind::Lock4], 4);
        assert_eq!(&loaded.0[3..], &[0; DeviceKind::ALL.len() - 3]);
    }

    #[test]
    fn counts_survive_a_save() {
        let mut counts = PerDevice::default();
        counts[DeviceKind::Summa6] = 7;
        let text = ron::to_string(&counts).unwrap();
        assert_eq!(ron::from_str::<PerDevice>(&text).unwrap(), counts);
    }
}
