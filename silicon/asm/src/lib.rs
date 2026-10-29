//! Assembler frontend shared by all ISAs.
//!
//! The syntax is the same for every ISA: labels, `;` comments, `.org`, `.db`,
//! decimal and `0x` numbers. Each ISA only provides an [`Encoder`].

mod assemble;
mod encoder;
mod labels;
mod number;
mod parse;

pub use assemble::assemble;
pub use encoder::Encoder;
pub use labels::Labels;
pub use number::parse_number;
pub use parse::{Instruction, Operand};
