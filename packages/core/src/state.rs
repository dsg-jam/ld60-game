use bevy::prelude::*;

/// Whether the game runs or waits behind a menu.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// The loading screen, shown at the start until the title screen is
    /// ready.
    #[default]
    Loading,
    /// The title screen.
    Title,
    Playing,
    Paused,
    /// The landlord closed the shop, so the run is over.
    Closed,
    /// The player bought the building, so the run is won.
    Owned,
}
