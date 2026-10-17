use crate::{Instruction, Labels};

pub trait Encoder {
    fn encode(&self, instruction: &Instruction, labels: &Labels) -> Result<Vec<u16>, String>;
    fn rom_len(&self) -> usize;
    fn size(&self, instruction: &Instruction) -> Result<usize, String>;
    fn word_bits(&self) -> u8;
}
