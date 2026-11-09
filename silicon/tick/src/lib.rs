//! Tick, the 4-bit chip in watch movements. See `docs/isa/tick.md`.
//!
//! # Features
//!
//! - `serde`: implements `Serialize` and `Deserialize` for [`Revision`].

mod cpu;
mod op;
mod revision;

pub use cpu::{Cpu, Effect};
pub use op::Op;
pub use revision::Revision;

/// Bits in one word of ROM.
pub const WORD_BITS: u8 = 4;
