use ld60_machine::{AsmError, Assembly, DisasmLine, Image, Isa};

use crate::Revision;
use crate::disasm::disassemble;
use crate::encoder::CalcoEncoder;

/// The CalCo instruction set of one chip revision. Get it with [`Revision::isa`].
#[derive(Debug)]
pub struct Calco(pub(crate) Revision);

impl Isa for Calco {
    fn name(&self) -> &'static str {
        match self.0 {
            Revision::A => "CalCo Rev A",
            Revision::B => "CalCo Rev B",
            Revision::C => "CalCo Rev C",
        }
    }

    fn mnemonics(&self) -> &'static [&'static str] {
        self.0.mnemonics()
    }

    fn assemble(&self, source: &str) -> Result<Assembly, Vec<AsmError>> {
        ld60_asm::assemble(source, &CalcoEncoder(self.0))
    }

    fn disassemble(&self, image: &Image) -> Vec<DisasmLine> {
        disassemble(image, self.0)
    }
}
