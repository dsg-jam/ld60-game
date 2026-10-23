use bevy::prelude::*;

/// Whether the game runs or waits behind a menu.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum AppState {
    /// The title screen, shown at the start.
    #[default]
    Title,
    Playing,
    Paused,
    /// The landlord closed the shop, so the run is over.
    Closed,
}
