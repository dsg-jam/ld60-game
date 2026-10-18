//! Devices that run firmware on an emulated chip, like the Summa6 adding
//! machine.
//!
//! # Features
//!
//! - `serde`: implements `Serialize` and `Deserialize` for the device and
//!   fault description types.

mod any_device;
mod board;
mod calco_device;
mod chip_spec;
mod device;
mod error;
mod fault;
mod isa_kind;
mod line_test;
mod readout;
mod rng;
mod rom;

pub use any_device::{AnyDevice, DeviceKind};
pub use board::{Annunciator, Board, BoardRev};
pub use calco_device::CalcoDevice;
pub use chip_spec::ChipSpec;
pub use device::{Device, MAX_STEPS};
pub use error::{DeviceError, ProgramError, RunError, UnknownBoardRev};
pub use fault::Fault;
pub use isa_kind::IsaKind;
pub use line_test::LineTest;
pub use readout::Readout;
pub use rom::{Rom, RomKind, can_program};
