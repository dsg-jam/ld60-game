//! An in-game console that calls the methods of the debug port.
//!
//! The key left of 1 opens it. `help` lists the commands.

mod complete;
mod console;
mod help;
mod line;
mod view;

use bevy::input::InputSystems;
use bevy::prelude::*;

use self::console::Console;
use self::view::View;

/// Needs [`DebugPortPlugin`](ld60_game_core::debug_port::DebugPortPlugin).
pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Console>()
            .add_systems(Startup, View::spawn)
            .add_systems(PreUpdate, Console::read_keys.before(InputSystems))
            .add_systems(Update, (Console::run, View::show).chain());
    }
}
