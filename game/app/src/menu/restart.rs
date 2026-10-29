use bevy::prelude::*;
use ld60_game_player::hints::{INK, SHADOW};
use ld60_locale::{Localized, Message};

use super::list::{Action, Entries};

const WARNING_SIZE: f32 = 22.0;

/// The entries below the warning.
const ENTRIES: &[(Action, &str)] = &[
    (Action::StartOver, "menu-start-over"),
    (Action::Back, "menu-back"),
];

/// Asks before the current run is given up for a new one.
pub(super) struct RestartPage;

impl RestartPage {
    pub(super) fn bundle() -> impl Bundle {
        (
            Node {
                flex_direction: FlexDirection::Column,
                ..default()
            },
            children![
                (
                    Localized::from(Message::new("menu-restart-warning")),
                    TextFont::from_font_size(FontSize::Px(WARNING_SIZE)),
                    TextColor(INK),
                    SHADOW,
                    Node {
                        margin: UiRect::bottom(px(WARNING_SIZE)),
                        ..default()
                    },
                ),
                Entries::bundle(ENTRIES),
            ],
        )
    }
}
