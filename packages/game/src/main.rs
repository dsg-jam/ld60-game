//! Bugs From Home, a Ludum Dare game.
//!
//! # Features
//!
//! - `debug_port`: makes a Bevy Remote Protocol server available for
//!   inspecting and driving the running game. It only starts when
//!   `LD60_DEBUG_PORT` is set to a port number, and only on native builds.

mod audio;
mod city;
#[cfg(feature = "debug_port")]
mod debug_port;
mod device;
mod layout;
mod menu;
mod parts;
mod player;
mod workshop;

use bevy::asset::AssetMetaCheck;
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;

const TITLE: &str = "Bugs From Home";
/// Meters per second squared, down.
const GRAVITY: f32 = 9.81;

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
        city::CityPlugin,
        ld60_geometry::GeometryPlugin,
        ld60_locale::LocalePlugin,
        menu::MenuPlugin,
        parts::PartsPlugin,
        player::PlayerPlugin,
        ld60_locale::TypefacePlugin,
        workshop::WorkshopPlugin,
        device::DevicePlugin,
    ));
    app.run()
}
