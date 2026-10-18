pub(crate) mod camera;
pub(crate) mod focus;
mod look;
mod shimmer;

use bevy::prelude::*;

/// The seated player: camera, focus, mouse look and hover shimmer.
pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraPlugin,
            focus::FocusPlugin,
            look::LookPlugin,
            shimmer::ShimmerPlugin,
        ));
    }
}
