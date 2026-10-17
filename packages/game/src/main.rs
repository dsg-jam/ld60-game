mod computer;
mod layout;
mod screen;
mod shape;
mod stage;

use bevy::asset::AssetMetaCheck;
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;

const TITLE: &str = "LD60";

fn main() -> AppExit {
    App::new()
        .add_plugins(
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
        )
        .add_plugins((
            MeshPickingPlugin,
            shape::ShapePlugin,
            screen::ScreenPlugin,
            computer::ComputerPlugin,
            stage::StagePlugin,
        ))
        .run()
}
