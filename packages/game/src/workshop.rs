mod clutter;
mod computer;
mod counter;
mod datasheet;
mod front;
pub(crate) mod job;
mod lamp;
mod pickup;
mod radio;
mod receipt;
mod rug;
mod service;
pub(crate) mod shop;
mod ticket;
mod tools;

use bevy::prelude::*;

use self::clutter::Clutter;
use self::computer::Computer;
use self::counter::{Counter, CounterLayout};
use self::datasheet::Datasheet;
use self::front::Front;
use self::lamp::Lamp;
use self::radio::Radio;
use self::shop::Shop;
use self::tools::Tool;
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
const RADIO_SPOT: Vec2 = Vec2::new(-0.5, -0.62);
const RADIO_YAW: f32 = 0.25;
const STAMP_SPOT: Vec2 = Vec2::new(0.1, -0.75);
const STAMP_YAW: f32 = 0.3;
const BELL_SPOT: Vec2 = Vec2::new(0.23, -0.75);
/// On the mat, see [`CounterLayout::place_on_mat`].
const DATASHEET_SPOT: Vec2 = Vec2::new(-0.69, -0.19);
const DATASHEET_YAW: f32 = 0.15;

pub(crate) struct WorkshopPlugin;

impl Plugin for WorkshopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            counter::CounterPlugin,
            clutter::ClutterPlugin,
            front::FrontPlugin,
            lamp::LampPlugin,
            radio::RadioPlugin,
            rug::RugPlugin,
            shop::ShopPlugin,
            datasheet::DatasheetPlugin,
            ticket::TicketPlugin,
            job::JobPlugin,
            receipt::ReceiptPlugin,
            tools::ToolsPlugin,
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
    let counter = CounterLayout::new();
    commands.spawn((Name::new("shop"), Shop));
    commands.spawn((Name::new("front"), Front));
    commands.spawn((Name::new("counter"), Counter));
    commands.spawn((Name::new("clutter"), Clutter));
    commands.spawn((
        Name::new("computer"),
        Computer,
        counter.place(COMPUTER_SPOT, COMPUTER_YAW),
    ));
    commands.spawn((
        Name::new("lamp"),
        Lamp::default(),
        counter.place(LAMP_SPOT, LAMP_YAW),
    ));
    commands.spawn((
        Name::new("radio"),
        Radio::default(),
        counter.place(RADIO_SPOT, RADIO_YAW),
    ));
    commands.spawn((
        Name::new("stamp"),
        Tool::Stamp,
        counter.place(STAMP_SPOT, STAMP_YAW),
    ));
    commands.spawn((Name::new("bell"), Tool::Bell, counter.place(BELL_SPOT, 0.0)));
    commands.spawn((
        Name::new("datasheet"),
        Datasheet,
        counter.place_on_mat(DATASHEET_SPOT, DATASHEET_YAW),
    ));
    commands.spawn((
        Name::new("camera"),
        PlayerCamera::seated_at(counter.seated_view()),
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
