pub mod camera;
mod crosshair;
mod ctrl;
pub mod cursor;
pub mod focus;
pub mod free_look;
pub mod hints;
pub mod lens;
mod logical_keys;
pub mod look;
mod refocus;
mod shimmer;
pub mod zoom;

use bevy::prelude::*;

pub use self::shimmer::NoShimmer;

/// The seated player: camera, crosshair, cursor, focus, key hints, mouse look,
/// free look and zoom while focused and hover shimmer.
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraPlugin,
            crosshair::CrosshairPlugin,
            ctrl::CtrlPlugin,
            cursor::CursorPlugin,
            focus::FocusPlugin,
            free_look::FreeLookPlugin,
            hints::HintsPlugin,
            lens::LensPlugin,
            logical_keys::LogicalKeysPlugin,
            look::LookPlugin,
            refocus::RefocusPlugin,
            shimmer::ShimmerPlugin,
            zoom::ZoomPlugin,
        ));
    }
}
