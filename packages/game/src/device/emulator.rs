use std::error::Error;

use bevy::prelude::*;
use ld60_devices::{AnyDevice, Device, MAX_STEPS, Readout, Rom};
use ld60_levels::TEST_SEEDS;

use super::{DeviceButton, DeviceKey};
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::parts::lcd::LcdDisplay;

pub(super) struct EmulatorPlugin;

impl Plugin for EmulatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Emulator::press_keys.after(DeviceButton::send_keys),
                Emulator::show_readout.after(Emulator::press_keys),
            ),
        );
        #[cfg(feature = "debug_port")]
        app.add_debug_method("ld60.readout", Emulator::debug_readout);
    }
}

/// The emulated chip and firmware inside a device. Its readout goes to the
/// device's [`LcdDisplay`].
#[derive(Component, Debug)]
#[require(LcdDisplay)]
pub(crate) struct Emulator {
    device: AnyDevice,
    readout: Readout,
    /// Index into [`TEST_SEEDS`] of the seed for the RAM garbage at the
    /// last power on. Each power on takes the next one, like the customer's
    /// tests do.
    power_ons: usize,
}

impl Emulator {
    /// Powers the device on and runs it until it waits for a key.
    pub(crate) fn power_on(device: AnyDevice) -> Self {
        Self::power_on_with(device, 0)
    }

    fn power_on_with(mut device: AnyDevice, power_ons: usize) -> Self {
        device.power_on(Self::seed(power_ons));
        Self::run(&mut device);
        let readout = device.readout();
        Self {
            device,
            readout,
            power_ons,
        }
    }

    /// Replaces the device and powers it on with fresh RAM garbage.
    pub(crate) fn restart(&mut self, device: AnyDevice) {
        *self = Self::power_on_with(device, self.power_ons + 1);
    }

    fn seed(power_ons: usize) -> u64 {
        TEST_SEEDS[power_ons % TEST_SEEDS.len()]
    }

    pub(crate) fn rom(&self) -> &Rom {
        self.device.rom()
    }

    fn run(device: &mut AnyDevice) {
        if let Err(err) = device.run_until_halt(MAX_STEPS) {
            warn!(error = &err as &dyn Error, "device did not halt");
        }
    }

    fn show_readout(mut emulators: Query<(&Emulator, &mut LcdDisplay), Changed<Emulator>>) {
        for (emulator, mut display) in &mut emulators {
            display.set_if_neq(LcdDisplay {
                digits: emulator.readout.digits.clone(),
                minus: emulator.readout.minus,
                error: emulator.readout.error,
            });
        }
    }

    fn press_keys(mut keys: MessageReader<DeviceKey>, mut emulators: Query<&mut Emulator>) {
        for key in keys.read().filter(|key| key.pressed) {
            let Ok(mut emulator) = emulators.get_mut(key.device) else {
                continue;
            };
            if let Err(err) = emulator.device.press_key(key.key) {
                warn!(error = &err as &dyn Error, "key press failed");
                continue;
            }
            Self::run(&mut emulator.device);
            emulator.readout = emulator.device.readout();
        }
    }

    /// `ld60.readout` returns the display of every emulated device by name.
    #[cfg(feature = "debug_port")]
    fn debug_readout(
        In(_): In<Option<serde_json::Value>>,
        emulators: Query<(&Name, &Emulator)>,
    ) -> bevy::remote::BrpResult {
        let readouts: serde_json::Map<_, _> = emulators
            .iter()
            .map(|(name, emulator)| (name.to_string(), emulator.readout.to_string().into()))
            .collect();
        Ok(readouts.into())
    }
}

#[cfg(test)]
mod tests {
    use ld60_levels::{Catalog, device, initial_rom};

    use super::*;

    fn tally() -> AnyDevice {
        let catalog = Catalog::on_disk().unwrap();
        let source = &catalog.levels()[0];
        let level = source.load().unwrap();
        let rom = initial_rom(&level, &source.buggy).unwrap();
        device(&level, rom).unwrap()
    }

    #[test]
    fn each_restart_takes_the_next_test_seed() {
        let mut emulator = Emulator::power_on(tally());
        let mut seeds = vec![Emulator::seed(emulator.power_ons)];
        for _ in 1..=TEST_SEEDS.len() {
            emulator.restart(tally());
            seeds.push(Emulator::seed(emulator.power_ons));
        }
        let mut expected = TEST_SEEDS.to_vec();
        expected.push(TEST_SEEDS[0]);
        assert_eq!(seeds, expected);
    }
}
