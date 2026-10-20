pub(crate) mod camera;
mod crosshair;
pub(crate) mod cursor;
pub(crate) mod focus;
pub(crate) mod free_look;
pub(crate) mod hints;
pub(crate) mod look;
mod shimmer;
pub(crate) mod zoom;

use bevy::prelude::*;

/// The seated player: camera, crosshair, cursor, focus, key hints, mouse look,
/// free look and zoom while focused and hover shimmer.
pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraPlugin,
            crosshair::CrosshairPlugin,
            cursor::CursorPlugin,
            focus::FocusPlugin,
            free_look::FreeLookPlugin,
            hints::HintsPlugin,
            look::LookPlugin,
            shimmer::ShimmerPlugin,
            zoom::ZoomPlugin,
        ));
    }
}
