pub(crate) mod clock;
mod clutter;
mod computer;
mod counter;
mod dial;
mod dust;
#[cfg(test)]
mod footprint;
mod front;
pub(crate) mod job;
mod lamp;
pub(crate) mod levels;
mod manuals;
mod pickup;
mod radio;
mod receipt;
mod reflections;
mod rent;
mod rug;
pub(crate) mod run;
mod salvage;
mod save;
mod service;
pub(crate) mod shop;
mod ticket;
mod tools;
mod tutorial;

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use self::clutter::Clutter;
use self::computer::Computer;
use self::counter::{Counter, CounterLayout};
use self::dial::Dial;
use self::front::Front;
use self::lamp::Lamp;
use self::manuals::ShopManuals;
use self::radio::Radio;
use self::shop::Shop;
use self::tools::Tool;
use crate::player::camera::PlayerCamera;

const COMPUTER_SPOT: Vec2 = Vec2::new(0.7, -0.3);
const COMPUTER_YAW: f32 = -0.4;
const LAMP_SPOT: Vec2 = Vec2::new(-0.8, -0.6);
const LAMP_YAW: f32 = -0.7;
const RADIO_SPOT: Vec2 = Vec2::new(-0.47, -0.62);
const RADIO_YAW: f32 = 0.25;
const STAMP_SPOT: Vec2 = Vec2::new(0.1, -0.75);
const STAMP_YAW: f32 = 0.3;
const BELL_SPOT: Vec2 = Vec2::new(0.23, -0.75);
/// Standing at the back, right of the radio.
const MANUALS_SPOT: Vec2 = Vec2::new(-0.19, -0.68);
const MANUALS_YAW: f32 = 0.15;
/// Lying open on the left part of the mat, see
/// [`CounterLayout::place_on_mat`].
const MANUALS_OPEN_SPOT: Vec2 = Vec2::new(-0.62, -0.4);
/// On the left wall, just behind the pillar of the shop front.
const DIAL_SPOT: Vec3 = Vec3::new(-1.45, 2.0, -0.05);

/// Where the player sits and where the things that make sounds on the
/// counter stand, in the world.
pub(crate) struct DeskSpots {
    /// The eyes and ears of the seated player.
    pub(crate) seat: Vec3,
    pub(crate) computer: Vec3,
    /// The manuals file, lying open.
    pub(crate) manuals: Vec3,
    pub(crate) lamp: Vec3,
}

impl DeskSpots {
    pub(crate) fn new() -> Self {
        let counter = CounterLayout::new();
        Self {
            seat: counter.seated_view().translation,
            computer: counter.place(COMPUTER_SPOT, 0.0).translation,
            manuals: counter.place_on_mat(MANUALS_OPEN_SPOT, 0.0).translation,
            lamp: counter.place(LAMP_SPOT, 0.0).translation,
        }
    }
}

pub(crate) struct WorkshopPlugin;

impl Plugin for WorkshopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            counter::CounterPlugin,
            dust::DustPlugin,
            clutter::ClutterPlugin,
            front::FrontPlugin,
            lamp::LampPlugin,
            radio::RadioPlugin,
            rug::RugPlugin,
            shop::ShopPlugin,
            manuals::ManualsPlugin,
            ticket::TicketPlugin,
            job::JobPlugin,
            levels::LevelsPlugin,
            receipt::ReceiptPlugin,
            tools::ToolsPlugin,
            computer::ComputerPlugin,
        ))
        .add_plugins((
            clock::ClockPlugin,
            dial::DialPlugin,
            reflections::ReflectionsPlugin,
            rent::RentPlugin,
            run::RunPlugin,
            salvage::SalvagePlugin,
            save::SavePlugin,
            tutorial::TutorialPlugin,
        ))
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
        Name::new("shop manuals"),
        ShopManuals {
            open_at: counter.place_on_mat(MANUALS_OPEN_SPOT, 0.0),
        },
        counter.place(MANUALS_SPOT, MANUALS_YAW),
    ));
    commands.spawn((
        Name::new("planet dial"),
        Dial,
        Transform::from_translation(DIAL_SPOT).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
    ));
    commands.spawn((
        Name::new("camera"),
        PlayerCamera::seated_at(counter.seated_view()),
    ));
}
