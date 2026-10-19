use std::collections::VecDeque;
use std::error::Error;

use bevy::prelude::*;
use ld60_devices::{AnyDevice, Device, MAX_STEPS, Readout, Rom};
use ld60_levels::TEST_SEEDS;
use ld60_machine::{Cpu, Step};

use super::{DeviceButton, DeviceKey};
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::parts::lcd::LcdDisplay;

/// How many instructions the trace remembers.
pub(crate) const TRACE_LEN: usize = 64;

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
    /// Set while the debugger holds the chip. It then runs only one step at
    /// a time.
    held: bool,
    /// Addresses where the chip stops and the debugger takes hold.
    breakpoints: Vec<u16>,
    /// The addresses of the last [`TRACE_LEN`] instructions the chip ran,
    /// oldest first. Each address shows up once.
    trace: VecDeque<u16>,
}

/// Why the chip stopped running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    /// It waits for a key.
    Halted,
    /// It reached a breakpoint and the debugger holds it.
    Break,
    /// It ran [`MAX_STEPS`] steps without waiting for a key.
    Timeout,
}

impl Emulator {
    /// Powers the device on and runs it until it waits for a key.
    pub(crate) fn power_on(device: AnyDevice) -> Self {
        Self::power_on_with(device, 0, false, Vec::new())
    }

    fn power_on_with(
        mut device: AnyDevice,
        power_ons: usize,
        held: bool,
        breakpoints: Vec<u16>,
    ) -> Self {
        device.power_on(Self::seed(power_ons));
        let readout = device.readout();
        let mut emulator = Self {
            device,
            readout,
            power_ons,
            held,
            breakpoints,
            trace: VecDeque::with_capacity(TRACE_LEN),
        };
        if !held {
            emulator.run(false);
        }
        emulator
    }

    /// Replaces the device and powers it on with fresh RAM garbage. A held
    /// chip stays held at its first instruction.
    pub(crate) fn restart(&mut self, device: AnyDevice) {
        let breakpoints = std::mem::take(&mut self.breakpoints);
        *self = Self::power_on_with(device, self.power_ons + 1, self.held, breakpoints);
    }

    fn seed(power_ons: usize) -> u64 {
        TEST_SEEDS[power_ons % TEST_SEEDS.len()]
    }

    pub(crate) fn rom(&self) -> &Rom {
        self.device.rom()
    }

    pub(crate) fn cpu(&self) -> &dyn Cpu {
        self.device.cpu()
    }

    pub(crate) fn held(&self) -> bool {
        self.held
    }

    /// Lets the debugger take hold of the chip. Key presses then wait for
    /// the next step.
    pub(crate) fn hold(&mut self) {
        self.held = true;
    }

    /// Runs one instruction of the held chip.
    pub(crate) fn step(&mut self) -> Step {
        let pc = self.device.cpu().pc();
        let step = self.device.step();
        if step == Step::Executed {
            self.record(pc);
        }
        self.readout = self.device.readout();
        step
    }

    /// Lets go of the chip and runs it until it waits for a key or reaches
    /// a breakpoint.
    pub(crate) fn release(&mut self) -> Stop {
        self.held = false;
        self.run(true)
    }

    pub(crate) fn breakpoints(&self) -> &[u16] {
        &self.breakpoints
    }

    pub(crate) fn set_breakpoints(&mut self, breakpoints: Vec<u16>) {
        self.breakpoints = breakpoints;
    }

    /// The addresses of the last instructions the chip ran, oldest first.
    pub(crate) fn trace(&self) -> impl Iterator<Item = u16> {
        self.trace.iter().copied()
    }

    /// Runs the chip until it stops. With `resume`, the instruction at the
    /// program counter runs even if it has a breakpoint, so the chip can
    /// leave one.
    fn run(&mut self, resume: bool) -> Stop {
        let stop = self.run_steps(resume);
        if stop == Stop::Timeout {
            warn!(steps = MAX_STEPS, "device did not halt");
        }
        self.readout = self.device.readout();
        stop
    }

    fn run_steps(&mut self, resume: bool) -> Stop {
        for steps in 0..MAX_STEPS {
            let pc = self.device.cpu().pc();
            if (steps > 0 || !resume) && self.breakpoints.contains(&pc) {
                self.held = true;
                return Stop::Break;
            }
            if self.device.step() == Step::Halted {
                return Stop::Halted;
            }
            self.record(pc);
        }
        Stop::Timeout
    }

    fn record(&mut self, addr: u16) {
        if self.trace.back() == Some(&addr) {
            return;
        }
        self.trace.retain(|&seen| seen != addr);
        if self.trace.len() == TRACE_LEN {
            self.trace.pop_front();
        }
        self.trace.push_back(addr);
    }

    fn show_readout(mut emulators: Query<(&Emulator, &mut LcdDisplay), Changed<Emulator>>) {
        for (emulator, mut display) in &mut emulators {
            display.set_if_neq(LcdDisplay {
                digits: emulator.readout.digits.clone(),
                minus: emulator.readout.minus,
                error: emulator.readout.error,
                open: emulator.readout.open,
                shut: emulator.readout.shut,
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
            if !emulator.held {
                emulator.run(true);
            }
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
    use ld60_levels::{device, initial_rom};

    use super::*;
    use crate::workshop::levels::Levels;

    fn tally() -> AnyDevice {
        let levels = Levels::on_disk();
        let source = &levels.levels()[0];
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

    #[test]
    fn traces_each_instruction_once() {
        let emulator = Emulator::power_on(tally());
        let trace: Vec<_> = emulator.trace().collect();
        assert!(!trace.is_empty());
        assert!(trace.len() <= TRACE_LEN);
        let mut unique = trace.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), trace.len());
    }

    #[test]
    fn a_held_chip_runs_only_when_stepped() {
        let mut emulator = Emulator::power_on(tally());
        let waiting = emulator.cpu().pc();
        emulator.hold();
        emulator.device.press_key('+').unwrap();
        assert_eq!(emulator.cpu().pc(), waiting);
        assert_eq!(emulator.step(), Step::Executed);
        assert_ne!(emulator.cpu().pc(), waiting);
        assert_eq!(emulator.release(), Stop::Halted);
        assert!(!emulator.held());
    }

    #[test]
    fn stops_at_breakpoints() {
        let mut emulator = Emulator::power_on(tally());
        let waiting = emulator.cpu().pc();
        emulator.device.press_key('+').unwrap();
        emulator.run(true);
        let breakpoints: Vec<_> = emulator.trace().filter(|&addr| addr != waiting).collect();
        emulator.set_breakpoints(breakpoints.clone());
        emulator.device.press_key('+').unwrap();
        assert_eq!(emulator.run(true), Stop::Break);
        assert!(emulator.held());
        assert!(breakpoints.contains(&emulator.cpu().pc()));
        emulator.set_breakpoints(Vec::new());
        assert_eq!(emulator.release(), Stop::Halted);
    }

    #[test]
    fn a_held_chip_stays_at_reset_after_a_restart() {
        let mut emulator = Emulator::power_on(tally());
        emulator.hold();
        emulator.restart(tally());
        assert!(emulator.held());
        assert_eq!(emulator.cpu().pc(), 0);
        assert_eq!(emulator.trace().count(), 0);
    }
}
