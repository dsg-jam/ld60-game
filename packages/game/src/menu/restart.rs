use bevy::prelude::*;

use super::list::{Action, Entries};
use crate::locale::{Localized, Message};
use crate::player::hints::{INK, SHADOW};

const WARNING_SIZE: f32 = 22.0;

/// The entries below the warning.
const ENTRIES: &[(Action, &str)] = &[
    (Action::NewRun, "menu-start-over"),
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
