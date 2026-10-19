use bevy::prelude::*;

use crate::locale::Message;
use crate::parts::paper::Paper;
use crate::parts::screen::UiCanvas;
use crate::player::focus::{Focus, FocusTarget, View};

/// To the right of the device, out of the way of the computer.
pub(super) const SPOT: Vec2 = Vec2::new(0.62, 0.55);
pub(super) const YAW: f32 = -0.2;
const SHEET_SIZE: Vec2 = Vec2::new(0.13, 0.17);
const CANVAS_SIZE: UVec2 = UVec2::new(560, 740);
const SHEET_GAP: f32 = 0.0005;
/// The paragraphs of each note, as attributes of its message.
const LESSONS: [&[&str]; 2] = [
    &["welcome", "ticket", "computer", "flash", "finish", "time"],
    &["datasheet", "lines", "versions", "rent", "trader"],
];

pub(crate) struct TutorialPlugin;

impl Plugin for TutorialPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(TutorialNote::on_add);
    }
}

/// A note from the human who ran the shop before, left next to a tutorial
/// job. Step `0` of the tutorial gets the first note.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct TutorialNote(pub(crate) usize);

impl TutorialNote {
    /// The note for step `step`, if it has one.
    pub(crate) fn for_step(step: usize) -> Option<impl Bundle> {
        (step < LESSONS.len()).then(|| {
            (
                Name::new("tutorial note"),
                Self(step),
                FocusTarget {
                    prompt: "click-note",
                    focus: Focus::Paper,
                    view: View::Local(Paper::reading_view(Self::sheet_center())),
                },
            )
        })
    }

    fn sheet_center() -> Vec3 {
        Vec3::Y * SHEET_GAP
    }

    fn message(self) -> Message {
        Message::new(format!("tutorial-{}", self.0 + 1))
    }

    fn on_add(
        add: On<Add<TutorialNote>>,
        notes: Query<&TutorialNote>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok(&note) = notes.get(add.entity) else {
            return;
        };
        let canvas = UiCanvas::spawn(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let page = Paper::spawn_page(
            &mut commands,
            &canvas,
            (1, 1),
            Message::new("tutorial-footer"),
            |page| note.write(page),
        );
        let mut sheet = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|note| {
            sheet = note
                .spawn(Paper::sheet(
                    &canvas,
                    SHEET_SIZE,
                    Self::sheet_center(),
                    &mut meshes,
                    &mut materials,
                ))
                .id();
        });
        commands
            .entity(add.entity)
            .insert(Paper::new(sheet, SHEET_SIZE, vec![page]));
    }

    fn write(self, page: &mut ChildSpawnerCommands) {
        let message = self.message();
        page.spawn(Paper::heading(message.clone().attribute("title")));
        for &paragraph in LESSONS[self.0] {
            page.spawn(Paper::text(message.clone().attribute(paragraph)));
        }
        page.spawn(Paper::faded(Message::new("tutorial-signed")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::{Locale, Localized};
    use crate::workshop::levels::Levels;

    #[test]
    fn every_note_formats() {
        let locale = Locale::on_disk();
        for step in 0..LESSONS.len() {
            let note = TutorialNote(step);
            locale.assert_formats(Localized::spawned_by(|page| note.write(page)));
        }
        locale.assert_formats([Message::new("tutorial-footer"), Message::new("click-note")]);
    }

    #[test]
    fn every_tutorial_job_has_a_note() {
        let steps = Levels::on_disk().tutorial().len();
        assert!((2..=3).contains(&steps));
        assert!((0..steps).all(|step| TutorialNote::for_step(step).is_some()));
        assert!(TutorialNote::for_step(steps).is_none());
    }
}
