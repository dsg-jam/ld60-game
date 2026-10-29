use bevy::{
    input_focus::tab_navigation::TabIndex,
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::TextInput,
};

use super::page::Page;
use super::undo::Undo;
use super::{PHOSPHOR, PHOSPHOR_DIM};

/// The player's own notes. They stay for the whole session.
#[derive(Component, Debug)]
pub(super) struct Notes;

impl Notes {
    /// The notes page, hidden until the player opens it.
    pub(super) fn bundle(font: TextFont) -> impl Bundle {
        (
            Notes,
            Page::Notes,
            Name::new("notes"),
            Node {
                flex_grow: 1.0,
                display: Display::None,
                ..default()
            },
            TextInput,
            EditableText {
                visible_lines: None,
                allow_newlines: true,
                ..default()
            },
            font,
            TextColor(PHOSPHOR),
            TextCursorStyle {
                color: PHOSPHOR,
                selection_color: PHOSPHOR_DIM,
                ..default()
            },
            TabIndex(0),
            Undo::default(),
        )
    }

    /// `ld60.notes` returns the text of the notes.
    #[cfg(feature = "debug_port")]
    pub(super) fn debug_state(
        In(_): In<Option<serde_json::Value>>,
        notes: Single<&EditableText, With<Notes>>,
    ) -> bevy::remote::BrpResult {
        Ok(serde_json::json!({ "text": notes.value().to_string() }))
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use ld60_game_run::job::Job;
    use ld60_game_run::levels::Levels;
    use ld60_locale::Localized;

    use super::*;
    use crate::editor::{Editor, dump::Dump, footer::Status, page::ActivePage, undo::Undo};

    #[test]
    fn notes_survive_a_new_job() {
        let mut world = World::new();
        world.init_resource::<Dump>();
        world.init_resource::<ActivePage>();
        world.spawn((Editor, EditableText::default(), Undo::default()));
        world.spawn((Status, Localized::verbatim("")));
        let notes = world
            .spawn((Notes, EditableText::new("ADD IS 3 BYTES")))
            .id();
        for index in [0, 1] {
            world.insert_resource(Job::load(&Levels::on_disk(), index).unwrap().unwrap());
            world.run_system_once(Editor::load_job).unwrap();
        }
        let text = world.get::<EditableText>(notes).unwrap();
        assert_eq!(text.value().to_string(), "ADD IS 3 BYTES");
    }
}
