mod calco;

pub(crate) use self::calco::CalcoBackend;
use crate::emit::{Emitter, Instr};
use crate::hir::Prim;
use crate::labels::Role;

/// What a chip can do, as far as legalization needs to know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Features {
    /// Routines can be called and return.
    pub(crate) calls: bool,
    /// `repeat` has a counted loop instruction.
    pub(crate) loops: bool,
    /// `arr[i]` has an indexed addressing mode.
    pub(crate) indexed: bool,
    pub(crate) ram_cells: usize,
    /// ROM size in words.
    pub(crate) rom_len: usize,
    pub(crate) stack_depth: Option<u8>,
}

/// How the assembler writes comments and labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Syntax {
    pub(crate) comment: &'static str,
    pub(crate) label_end: char,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Port {
    pub(crate) name: &'static str,
    pub(crate) output: bool,
}

/// Lowers primitives to the instructions of one instruction set.
///
/// The primitives assume an accumulator machine with one index register that
/// also counts loops. A machine of another shape, like a stack machine or one
/// with several general registers, needs changes to the primitives first.
pub(crate) trait Backend {
    /// The RAM cell named `name` in a `cell` declaration, like `M7`.
    fn cell(&self, name: &str) -> Option<u8>;
    /// How the header names the cell at `addr`.
    fn cell_name(&self, addr: u8) -> String;
    fn features(&self) -> &Features;
    /// Flags usable in conditions. `carry` may also be set to 0 or 1.
    fn flags(&self) -> &[&'static str];
    /// Formats an instruction with its operands already printed.
    fn format(&self, instr: &Instr, operands: &[&str]) -> String;
    /// The offset for an array at `first`, where the index starts at 1.
    fn index_offset(&self, first: u8) -> u8;
    /// The cell that `offset` reaches with the index at `index`.
    fn indexed_cell(&self, offset: u8, index: u8) -> u8;
    /// Names usable as `name()` statements.
    fn intrinsics(&self) -> &[&'static str];
    /// What a join label in front of `instr` is named after.
    fn label_role(&self, instr: &Instr) -> Role;
    /// Lowers one primitive. Legalization already ran.
    fn lower(&self, prim: &Prim, out: &mut Emitter) -> Result<(), String>;
    fn name(&self) -> &'static str;
    fn ports(&self) -> &[Port];
    fn syntax(&self) -> Syntax;
}
