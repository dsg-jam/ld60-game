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
    pub(crate) ram_cells: u8,
    pub(crate) stack_depth: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Port {
    pub(crate) name: &'static str,
    pub(crate) output: bool,
}

/// Lowers primitives to the instructions of one instruction set.
pub(crate) trait Backend {
    fn features(&self) -> &Features;
    /// Names usable as `name()` statements.
    fn intrinsics(&self) -> &[&'static str];
    /// What a join label in front of `instr` is named after.
    fn label_role(&self, instr: &Instr) -> Role;
    /// Lowers one primitive. Legalization already ran.
    fn lower(&self, prim: &Prim, out: &mut Emitter) -> Result<(), String>;
    fn name(&self) -> &'static str;
    fn ports(&self) -> &[Port];
}
