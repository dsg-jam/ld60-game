//! Procedural props that dress the shelves of the shop.

mod group;
mod prop;
mod shelf;
mod writing;

pub use self::shelf::Theme;
use bevy::prelude::*;

/// The writing the props share.
pub struct PropsPlugin;

impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(writing::WritingPlugin);
    }
}
