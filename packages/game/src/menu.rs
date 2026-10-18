mod pause;

use bevy::prelude::*;

pub(crate) struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>().add_plugins(pause::PausePlugin);
    }
}

/// Whether the game runs or waits behind a menu.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum AppState {
    #[default]
    Playing,
    Paused,
}
