//! CalCo calculator coprocessor. See `docs/isa/calco.md`.
//!
//! # Features
//!
//! - `serde`: implements `Serialize` and `Deserialize` for the chip
//!   description types.

mod chip;
mod cpu;
mod disasm;
mod encoder;
mod errata;
mod isa;
mod op;
pub mod ports;

pub use chip::{ChipInfo, DateCode, Revision};
pub use cpu::{Cpu, RAM_LEN, ROM_LEN};
pub use errata::{Errata, Erratum};
pub use isa::Calco;
pub use op::{Addr, Cond, Op};
