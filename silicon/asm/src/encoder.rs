use ld60_machine::AsmErrorKind;

use crate::{Instruction, Labels};

pub trait Encoder {
    fn encode(&self, instruction: &Instruction, labels: &Labels) -> Result<Vec<u16>, AsmErrorKind>;
    fn rom_len(&self) -> usize;
    fn size(&self, instruction: &Instruction) -> Result<usize, AsmErrorKind>;
    fn word_bits(&self) -> u8;
}
