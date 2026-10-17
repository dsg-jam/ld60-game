use bevy::prelude::*;

use crate::computer::Computer;

const BACKGROUND: Color = Color::srgb_u8(0x2a, 0x26, 0x24);

pub(crate) struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(BACKGROUND))
            .add_systems(Startup, spawn_stage);
    }
}

fn spawn_stage(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Computer::new().overview()));
    commands.spawn((
        DirectionalLight {
            illuminance: 4_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
