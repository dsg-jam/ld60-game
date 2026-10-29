use bevy::prelude::*;
use ld60_game_core::player::hints::{INK, SHADOW};
use ld60_locale::{Localized, Message};

use super::list::{Action, Entries};
use super::{AppState, Column};
use crate::workshop::run::Stats;

const STATS_SIZE: f32 = 22.0;
const STATS_GAP: f32 = 6.0;
const SHADE: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);

/// The entries of the screen after the shop closed.
pub(super) const ENTRIES: &[(Action, &str)] = &[
    (Action::NewRun, "menu-new-run"),
    #[cfg(not(target_arch = "wasm32"))]
    (Action::Quit, "menu-quit"),
];

/// The entries of the screen after the player bought the building.
pub(super) const OWNED_ENTRIES: &[(Action, &str)] = &[
    (Action::Resume, "menu-keep-open"),
    (Action::NewRun, "menu-new-shop"),
    #[cfg(not(target_arch = "wasm32"))]
    (Action::Quit, "menu-quit"),
];

pub(super) struct ClosedPlugin;

impl Plugin for ClosedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Closed), spawn_closed)
            .add_systems(OnEnter(AppState::Owned), spawn_owned);
    }
}

/// What the shop did before it closed.
pub(super) struct StatsPage;

impl StatsPage {
    pub(super) fn bundle(stats: Stats) -> impl Bundle {
        let line = |message: Message| {
            (
                Localized::from(message),
                TextFont::from_font_size(FontSize::Px(STATS_SIZE)),
                TextColor(INK),
                SHADOW,
            )
        };
        (
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(STATS_GAP),
                margin: UiRect::bottom(px(STATS_SIZE)),
                ..default()
            },
            children![
                line(Message::new("closed-cycles").with("cycles", stats.cycles)),
                line(Message::new("closed-fixed").with("jobs", stats.fixed)),
                line(Message::new("closed-lost").with("jobs", stats.lost)),
                line(Message::new("closed-earned").with("amount", stats.earned)),
                line(Message::new("closed-rent").with("amount", stats.rent)),
                line(Message::new("closed-spent").with("amount", stats.spent)),
            ],
        )
    }

    pub(super) fn with_entries(stats: Stats, entries: &'static [(Action, &str)]) -> impl Bundle {
        (
            Node {
                flex_direction: FlexDirection::Column,
                ..default()
            },
            children![Self::bundle(stats), Entries::bundle(entries)],
        )
    }
}

fn spawn_closed(mut commands: Commands) {
    spawn(&mut commands, AppState::Closed, "menu-closed");
}

fn spawn_owned(mut commands: Commands) {
    spawn(&mut commands, AppState::Owned, "menu-owned");
}

/// Darkens the shop behind the screen and shows `heading`.
fn spawn(commands: &mut Commands, state: AppState, heading: &'static str) {
    commands.spawn((
        DespawnOnExit(state),
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(SHADE),
    ));
    commands.spawn(Column::screen(state, Message::new(heading)));
}
