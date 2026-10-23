use bevy::prelude::*;

use super::counter::CounterLayout;

pub const COMPUTER_SPOT: Vec2 = Vec2::new(0.7, -0.3);
pub const LAMP_SPOT: Vec2 = Vec2::new(-0.8, -0.6);
/// Lying open on the left part of the mat, see
/// [`CounterLayout::place_on_mat`].
pub const MANUALS_OPEN_SPOT: Vec2 = Vec2::new(-0.62, -0.4);

/// Where the player sits and where the things that make sounds on the
/// counter stand, in the world.
pub struct DeskSpots {
    /// The eyes and ears of the seated player.
    pub seat: Vec3,
    pub computer: Vec3,
    /// The manuals file, lying open.
    pub manuals: Vec3,
    pub lamp: Vec3,
}

impl Default for DeskSpots {
    fn default() -> Self {
        Self::new()
    }
}

impl DeskSpots {
    pub fn new() -> Self {
        let counter = CounterLayout::new();
        Self {
            seat: counter.seated_view().translation,
            computer: counter.place(COMPUTER_SPOT, 0.0).translation,
            manuals: counter.place_on_mat(MANUALS_OPEN_SPOT, 0.0).translation,
            lamp: counter.place(LAMP_SPOT, 0.0).translation,
        }
    }
}
