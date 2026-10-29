use crate::{AsmError, Assembly, DisasmLine, Image};

pub trait Isa: Sync {
    fn assemble(&self, source: &str) -> Result<Assembly, Vec<AsmError>>;
    fn disassemble(&self, image: &Image) -> Vec<DisasmLine>;
    fn mnemonics(&self) -> &'static [&'static str];
    fn name(&self) -> &'static str;
    /// Port names that operands can use, in upper case.
    fn ports(&self) -> &'static [&'static str];
    /// Register and memory cell names that operands can use, in upper case.
    fn registers(&self) -> &'static [&'static str];
}
