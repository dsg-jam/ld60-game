//! Building blocks shared by the emulated chips: CPU and bus traits, RAM,
//! program images and assembly listings.
//!
//! # Features
//!
//! - `serde`: implements `Serialize` and `Deserialize` for [`StuckBit`].

mod assembly;
mod bus;
mod cpu;
mod image;
mod isa;
mod ram;

pub use assembly::{AsmError, Assembly, DisasmLine, SourceLine};
pub use bus::Bus;
pub use cpu::{Cpu, Register, Step};
pub use image::Image;
pub use isa::Isa;
pub use ram::{Ram, StuckBit};
