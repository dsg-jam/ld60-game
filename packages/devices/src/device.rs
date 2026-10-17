use machine::{Cpu, Step};

use crate::{DeviceKind, Fault, IsaKind, Readout, Rom, RunError};

pub const MAX_STEPS: u32 = 100_000;

pub trait Device {
    fn apply_fault(&mut self, fault: Fault) {
        match fault {
            Fault::StuckRamBit(bit) => self.cpu_mut().ram_mut().add_stuck_bit(bit),
            Fault::FadedRomBits { addr, mask } => self.rom_mut().fade(usize::from(addr), mask),
        }
    }

    fn cpu(&self) -> &dyn Cpu;
    fn cpu_mut(&mut self) -> &mut dyn Cpu;
    fn isa(&self) -> IsaKind;
    fn key_code(&self, key: char) -> Option<u8>;
    fn kind(&self) -> DeviceKind;
    /// Fills RAM with garbage derived from `seed` and resets the CPU.
    fn power_on(&mut self, seed: u64);
    fn press(&mut self, code: u8);

    fn press_key(&mut self, key: char) -> Result<(), RunError> {
        let code = self.key_code(key).ok_or(RunError::UnknownKey(key))?;
        self.press(code);
        Ok(())
    }

    fn readout(&self) -> Readout;
    fn rom(&self) -> &Rom;
    fn rom_mut(&mut self) -> &mut Rom;

    fn run_until_halt(&mut self, max_steps: u32) -> Result<u32, RunError> {
        for steps in 0..max_steps {
            if self.step() == Step::Halted {
                return Ok(steps);
            }
        }
        Err(RunError::Timeout { steps: max_steps })
    }

    fn step(&mut self) -> Step;
}
