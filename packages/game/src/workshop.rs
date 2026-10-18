mod computer;
mod desk;
mod lamp;

use bevy::prelude::*;

use self::computer::Computer;
use self::desk::{Desk, DeskLayout};
use self::lamp::Lamp;
use crate::device::Summa6;
use crate::player::camera::PlayerCamera;

const BACKGROUND: Color = Color::srgb_u8(0x2a, 0x26, 0x24);

const COMPUTER_SPOT: Vec2 = Vec2::new(0.55, -0.3);
const COMPUTER_YAW: f32 = -0.5;
const LAMP_SPOT: Vec2 = Vec2::new(-0.8, -0.6);
const LAMP_YAW: f32 = -0.7;
const SUMMA6_SPOT: Vec2 = Vec2::new(-0.05, 0.3);
const SUMMA6_YAW: f32 = 0.15;

pub(crate) struct WorkshopPlugin;

impl Plugin for WorkshopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((desk::DeskPlugin, lamp::LampPlugin, computer::ComputerPlugin))
            .insert_resource(ClearColor(BACKGROUND))
            .add_systems(Startup, spawn_stage);
    }
}

fn spawn_stage(mut commands: Commands) {
    let desk = DeskLayout::new();
    commands.spawn((Name::new("desk"), Desk));
    commands.spawn((
        Name::new("computer"),
        Computer,
        desk.place(COMPUTER_SPOT, COMPUTER_YAW),
    ));
    commands.spawn((Name::new("lamp"), Lamp, desk.place(LAMP_SPOT, LAMP_YAW)));
    commands.spawn((
        Name::new("summa6"),
        Summa6::for_level(&ld60_levels::CATALOG[0]),
        desk.place(SUMMA6_SPOT, SUMMA6_YAW),
    ));
    commands.spawn((
        Name::new("camera"),
        PlayerCamera::seated_at(desk.seated_view()),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 4_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
