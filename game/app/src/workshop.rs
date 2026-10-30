mod bin;
mod cable;
mod clutter;
mod counter;
mod delivery;
mod dial;
mod footprint;
mod front;
mod lamp;
mod printer;
mod radio;
mod receipt;
mod reflections;
mod reminder;
mod rent;
mod rug;
mod salvage;
mod save;
mod shop;
mod stamp;
mod ticket;
mod till;

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use ld60_game_computer::Computer;
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_core::layout::desk::{COMPUTER_SPOT, LAMP_SPOT, MANUALS_OPEN_SPOT};
use ld60_game_manuals::ShopManuals;
use ld60_game_player::camera::PlayerCamera;
use ld60_game_player::focus::Focused;
use ld60_game_run::NewRun;

use self::bin::Bin;
use self::clutter::Clutter;
use self::counter::Counter;
use self::dial::Dial;
use self::front::Front;
use self::lamp::Lamp;
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
            clutter::ClutterPlugin,
            front::FrontPlugin,
            lamp::LampPlugin,
            radio::RadioPlugin,
            rug::RugPlugin,
            shop::ShopPlugin,
            ld60_game_manuals::ManualsPlugin,
            ticket::TicketPlugin,
            ld60_game_run::job::JobPlugin,
            ld60_game_run::levels::LevelsPlugin,
            receipt::ReceiptPlugin,
            stamp::StampPlugin,
            ld60_game_computer::ComputerPlugin,
        ))
        .add_plugins((
            cable::CablePlugin,
            ld60_game_run::chore::ChorePlugin,
            delivery::DeliveryPlugin,
            dial::DialPlugin,
            printer::PrinterPlugin,
            reminder::ReminderPlugin,
            bin::BinPlugin,
            reflections::ReflectionsPlugin,
            rent::RentPlugin,
            ld60_game_run::RunPlugin,
            salvage::SalvagePlugin,
            save::SavePlugin,
            till::TillPlugin,
            ld60_game_run::tutorial::TutorialPlugin,
        ))
        .add_systems(Startup, spawn_stage)
        .add_observer(Focused::leave_on::<NewRun>);
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
        counter.place(delivery::TICKET_SPOT, delivery::TICKET_YAW),
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
