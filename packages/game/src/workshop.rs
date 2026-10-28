mod bin;
mod cable;
mod chore;
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
mod printer;
mod radio;
mod receipt;
mod reflections;
mod regulars;
mod reminder;
mod rent;
mod rug;
pub(crate) mod run;
mod salvage;
mod save;
mod service;
mod shop;
mod stamp;
mod ticket;
mod till;
mod tutorial;

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use ld60_core::layout::counter::CounterLayout;
use ld60_core::layout::desk::{COMPUTER_SPOT, LAMP_SPOT, MANUALS_OPEN_SPOT};
use ld60_core::player::camera::PlayerCamera;

use self::bin::Bin;
use self::clutter::Clutter;
use self::computer::Computer;
use self::counter::Counter;
use self::dial::Dial;
use self::front::Front;
use self::lamp::Lamp;
use self::manuals::ShopManuals;
use self::printer::Printer;
use self::radio::Radio;
use self::reminder::Reminder;
use self::shop::Shop;
use self::stamp::RubberStamp;
use self::ticket::Clipboard;
use self::till::Till;

const COMPUTER_YAW: f32 = -0.4;
const LAMP_YAW: f32 = -0.7;
const RADIO_SPOT: Vec2 = Vec2::new(-0.47, -0.62);
const RADIO_YAW: f32 = 0.25;
const STAMP_SPOT: Vec2 = Vec2::new(0.17, -0.75);
const STAMP_YAW: f32 = 0.3;
/// Standing at the back, right of the radio.
const MANUALS_SPOT: Vec2 = Vec2::new(-0.19, -0.68);
const MANUALS_YAW: f32 = 0.15;
/// On the left wall, just behind the pillar of the shop front.
const DIAL_SPOT: Vec3 = Vec3::new(-1.45, 2.0, -0.05);

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
            stamp::StampPlugin,
            computer::ComputerPlugin,
        ))
        .add_plugins((
            cable::CablePlugin,
            chore::ChorePlugin,
            dial::DialPlugin,
            printer::PrinterPlugin,
            reminder::ReminderPlugin,
            bin::BinPlugin,
            reflections::ReflectionsPlugin,
            rent::RentPlugin,
            run::RunPlugin,
            salvage::SalvagePlugin,
            save::SavePlugin,
            till::TillPlugin,
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
        RubberStamp,
        counter.place(STAMP_SPOT, STAMP_YAW),
    ));
    commands.spawn((
        Name::new("clipboard"),
        Clipboard,
        counter.place(job::TICKET_SPOT, job::TICKET_YAW),
    ));
    commands.spawn((Name::new("paper bin"), Bin, Bin::place()));
    commands.spawn((
        Name::new("rent reminder"),
        Reminder,
        counter.place(reminder::SPOT, reminder::YAW),
    ));
    commands.spawn((
        Name::new("ticket printer"),
        Printer,
        counter.place(printer::SPOT, printer::YAW),
    ));
    commands.spawn((
        Name::new("till"),
        Till::default(),
        counter.place(till::SPOT, till::YAW),
    ));
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
