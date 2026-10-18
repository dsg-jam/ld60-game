use std::error::Error;

use bevy::prelude::*;
use ld60_devices::{AnyDevice, Device, MAX_STEPS, Readout, Rom};

use super::{DeviceButton, DeviceKey};
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::parts::lcd::LcdDisplay;

/// Decides the random RAM contents at power on.
const POWER_ON_SEED: u64 = 1;

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
}

impl Emulator {
    /// Powers the device on and runs it until it waits for a key.
    pub(crate) fn power_on(mut device: AnyDevice) -> Self {
        device.power_on(POWER_ON_SEED);
        Self::run(&mut device);
        let readout = device.readout();
        Self { device, readout }
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
