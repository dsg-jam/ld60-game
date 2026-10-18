use ld60_machine::{AsmError, Assembly, DisasmLine, Image, Isa};

use crate::disasm::disassemble;
use crate::encoder::{CalcoEncoder, MNEMONICS};

pub struct Calco;

impl Isa for Calco {
    fn name(&self) -> &'static str {
        "CalCo"
    }

    fn mnemonics(&self) -> &'static [&'static str] {
        MNEMONICS
    }

    fn assemble(&self, source: &str) -> Result<Assembly, Vec<AsmError>> {
        ld60_asm::assemble(source, &CalcoEncoder)
    }

    fn disassemble(&self, image: &Image) -> Vec<DisasmLine> {
        disassemble(image)
    }
}
