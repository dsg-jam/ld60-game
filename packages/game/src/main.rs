//! A LD60 game.
//!
//! # Features
//!
//! - `debug_port`: makes a Bevy Remote Protocol server available for
//!   inspecting and driving the running game. It only starts when
//!   `LD60_DEBUG_PORT` is set to a port number, and only on native builds.

mod audio;
#[cfg(feature = "debug_port")]
mod debug_port;
mod device;
mod geometry;
mod parts;
mod player;
mod workshop;

use bevy::asset::AssetMetaCheck;
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;

const TITLE: &str = "LD60";

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: TITLE.to_owned(),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            }),
    );
    #[cfg(feature = "debug_port")]
    app.add_plugins(debug_port::DebugPortPlugin);
    app.add_plugins((
        MeshPickingPlugin,
        audio::AudioPlugin,
        geometry::GeometryPlugin,
        parts::PartsPlugin,
        player::PlayerPlugin,
        workshop::WorkshopPlugin,
        device::DevicePlugin,
    ));
    app.run()
}
