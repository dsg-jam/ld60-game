mod clutter;
mod computer;
mod desk;
mod lamp;
mod room;
mod switch;

use bevy::prelude::*;

use self::clutter::Clutter;
use self::computer::Computer;
use self::desk::{Desk, DeskLayout};
use self::lamp::Lamp;
use self::room::Room;
use crate::device::Summa6;
use crate::player::camera::PlayerCamera;

const BACKGROUND: Color = Color::srgb_u8(0x14, 0x12, 0x11);
/// Evening light from the window, much weaker than the desk lamp.
const MOONLIGHT: Color = Color::srgb(0.65, 0.75, 1.0);
const MOONLIGHT_ILLUMINANCE: f32 = 350.0;
const AMBIENT: Color = Color::srgb(1.0, 0.85, 0.7);
/// Stands in for light from the ceiling lamp bouncing off the walls.
const AMBIENT_BRIGHTNESS: f32 = 90.0;

const COMPUTER_SPOT: Vec2 = Vec2::new(0.55, -0.3);
const COMPUTER_YAW: f32 = -0.5;
const LAMP_SPOT: Vec2 = Vec2::new(-0.8, -0.6);
const LAMP_YAW: f32 = -0.7;
const SUMMA6_SPOT: Vec2 = Vec2::new(-0.05, 0.3);
const SUMMA6_YAW: f32 = 0.15;

pub(crate) struct WorkshopPlugin;

impl Plugin for WorkshopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            desk::DeskPlugin,
            clutter::ClutterPlugin,
            lamp::LampPlugin,
            room::RoomPlugin,
            switch::SwitchPlugin,
            computer::ComputerPlugin,
        ))
        .insert_resource(ClearColor(BACKGROUND))
        .insert_resource(GlobalAmbientLight {
            color: AMBIENT,
            brightness: AMBIENT_BRIGHTNESS,
            ..default()
        })
        .add_systems(Startup, spawn_stage);
    }
}

fn spawn_stage(mut commands: Commands) {
    let desk = DeskLayout::new();
    commands.spawn((Name::new("room"), Room));
    commands.spawn((Name::new("desk"), Desk));
    commands.spawn((Name::new("clutter"), Clutter));
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
            color: MOONLIGHT,
            illuminance: MOONLIGHT_ILLUMINANCE,
            // Its shadows follow the camera and swim over small things.
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-1.5, 2.0, -1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
