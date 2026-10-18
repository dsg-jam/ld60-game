pub(crate) mod camera;
mod crosshair;
pub(crate) mod cursor;
pub(crate) mod focus;
mod hints;
pub(crate) mod look;
mod shimmer;

use bevy::prelude::*;

/// The seated player: camera, crosshair, cursor, focus, key hints, mouse look
/// and hover shimmer.
pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraPlugin,
            crosshair::CrosshairPlugin,
            cursor::CursorPlugin,
            focus::FocusPlugin,
            hints::HintsPlugin,
            look::LookPlugin,
            shimmer::ShimmerPlugin,
        ));
    }
}
