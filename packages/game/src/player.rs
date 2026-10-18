pub(crate) mod camera;
pub(crate) mod focus;
mod look;

use bevy::prelude::*;

/// The seated player: camera, focus and mouse look.
pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((camera::CameraPlugin, focus::FocusPlugin, look::LookPlugin));
    }
}
