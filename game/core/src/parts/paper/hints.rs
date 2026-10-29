use bevy::prelude::*;
use ld60_locale::Message;

use super::{Paper, ShowsOwnPages};
use crate::player::hints::{Hint, Hints, Icon, Input};

impl Paper {
    /// Papers with more than one page say how to turn them. A paper that
    /// shows its own pages, like the manuals file, sets its own hints.
    #[expect(clippy::type_complexity, reason = "Bevy query")]
    pub(super) fn show_page_hints(
        papers: Query<(Entity, &Paper, Option<&Hints>), (Changed<Paper>, Without<ShowsOwnPages>)>,
        mut commands: Commands,
    ) {
        for (entity, paper, hints) in &papers {
            let wanted = if paper.page_count() > 1 {
                Hints(vec![Hint {
                    inputs: vec![Input::Icon(Icon::Wheel), Input::Icon(Icon::ArrowsSideways)],
                    action: Message::new("hint-turn-pages"),
                }])
            } else {
                Hints::default()
            };
            if hints != Some(&wanted) {
                commands.entity(entity).insert(wanted);
            }
        }
    }
}
