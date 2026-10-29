use crate::{Bus, Ram};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Executed,
    Halted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Register {
    pub name: &'static str,
    pub value: u16,
    pub bits: u8,
}

pub trait Cpu {
    fn pc(&self) -> u16;
    fn ram(&self) -> &Ram;
    fn ram_mut(&mut self) -> &mut Ram;
    fn registers(&self) -> Vec<Register>;
    fn reset(&mut self);
    fn step(&mut self, bus: &mut dyn Bus) -> Step;
}
