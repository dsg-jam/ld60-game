//! The seated player, the parts the game is built from, its sound, and the
//! shop layout, clock, state and settings that the workshop and the city share.
//!
//! # Features
//!
//! - `debug_port`: adds the Bevy Remote Protocol server in [`debug_port`].
//! - `test-support`: adds helpers for tests in other crates.

pub mod audio;
pub mod clock;
#[cfg(feature = "debug_port")]
pub mod debug_port;
pub mod layout;
pub mod parts;
pub mod player;
pub mod settings;
pub mod state;

/// Meters per second squared, down.
pub const GRAVITY: f32 = 9.81;
