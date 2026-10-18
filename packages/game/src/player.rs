pub(crate) mod camera;
pub(crate) mod cursor;
pub(crate) mod focus;
mod look;
mod shimmer;

use bevy::prelude::*;

/// The seated player: camera, cursor, focus, mouse look and hover shimmer.
pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraPlugin,
            cursor::CursorPlugin,
            focus::FocusPlugin,
            look::LookPlugin,
            shimmer::ShimmerPlugin,
        ));
    }
}
