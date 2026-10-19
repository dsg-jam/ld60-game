use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::run::Run;
use crate::locale::Message;
use crate::menu::AppState;
use crate::parts::paper::Paper;
use crate::parts::screen::UiCanvas;
use crate::player::camera::PlayerCamera;
use crate::player::focus::{Focus, FocusOn, FocusTarget, Focused, View};

/// On the back left part of the mat, far enough from the device to keep it
/// out of sight while reading. Relative to the mat, with the whole note on
/// it.
pub(super) const SPOT: Vec2 = Vec2::new(-0.62, -0.45);
pub(super) const YAW: f32 = 0.08;
pub(super) const SHEET_SIZE: Vec2 = Vec2::new(0.15, 0.215);
const CANVAS_SIZE: UVec2 = UVec2::new(450, 645);
const SHEET_GAP: f32 = 0.0005;
/// The paragraphs of each note, as attributes of its message.
const LESSONS: [&[&str]; 2] = [
    &["welcome", "ticket", "computer", "flash", "try", "finish"],
    &["datasheet", "lines", "versions", "rent", "trader"],
];

pub(crate) struct TutorialPlugin;

impl Plugin for TutorialPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(TutorialNote::on_add)
            .add_observer(TutorialNote::mark_read)
            .add_systems(
                Update,
                (
                    TutorialNote::open_unread.run_if(in_state(Focus::Desk)),
                    TutorialNote::file_read,
                )
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

/// A note from the human who ran the shop before, left next to a tutorial
/// job. Step `0` of the tutorial gets the first note.
///
/// Once the player has read it and put it down, it goes into the manuals
/// file.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[require(Transform, Visibility)]
pub(crate) struct TutorialNote(pub(crate) usize);

/// Marks the first note until it opens on its own.
#[derive(Component, Debug)]
struct Unread;

/// Marks a note the player has opened.
#[derive(Component, Debug)]
struct Read;

impl TutorialNote {
    /// The note of step `step`, if it has one.
    pub(super) fn new(step: usize) -> Option<Self> {
        (step < LESSONS.len()).then_some(Self(step))
    }

    /// The note for step `step` on the counter, if it has one.
    pub(crate) fn for_step(step: usize) -> Option<impl Bundle> {
        Self::new(step).map(|note| {
            (
                Name::new("tutorial note"),
                note,
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

    pub(super) fn title(self) -> Message {
        self.message().attribute("title")
    }

    /// The short name of the note in the index of the manuals file.
    pub(super) fn label(self) -> Message {
        self.message().attribute("label")
    }

    pub(super) fn footer(self) -> Message {
        Message::new("tutorial-footer")
    }

    /// The notes of the steps before `step`.
    pub(super) fn before(step: usize) -> impl Iterator<Item = Self> {
        (0..step.min(LESSONS.len())).map(Self)
    }

    fn on_add(
        add: On<Add<TutorialNote>>,
        notes: Query<&TutorialNote>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        let Ok(&note) = notes.get(add.entity) else {
            return;
        };
        if note.0 == 0 {
            commands.entity(add.entity).insert(Unread);
        }
        let canvas = UiCanvas::spawn_print(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let page = Paper::spawn_single(&mut commands, &canvas, note.footer(), |page| {
            note.write(page);
        });
        let document = ("tutorial", note.0);
        let mut sheet = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|note| {
            sheet = note
                .spawn(Paper::sheet(
                    &canvas,
                    document,
                    SHEET_SIZE,
                    Self::sheet_center(),
                    &mut meshes,
                ))
                .id();
        });
        commands
            .entity(add.entity)
            .insert(Paper::new(sheet, SHEET_SIZE, vec![page]));
    }

    pub(super) fn write(self, page: &mut ChildSpawnerCommands) {
        let message = self.message();
        page.spawn(Paper::heading(self.title()));
        for &paragraph in LESSONS[self.0] {
            page.spawn(Paper::text(message.clone().attribute(paragraph)));
        }
        page.spawn(Paper::faded(Message::new("tutorial-signed")));
    }

    /// Waits until the player sits at the bench, so the camera does not
    /// swing to the note on its way in from the street.
    fn open_unread(
        notes: Query<Entity, With<Unread>>,
        camera: Single<(&PlayerCamera, &Transform)>,
        mut commands: Commands,
    ) {
        let (camera, at) = *camera;
        if !camera.seated(at) {
            return;
        }
        for note in &notes {
            commands.entity(note).remove::<Unread>();
            commands.trigger(FocusOn { entity: note });
        }
    }

    fn mark_read(
        focus_on: On<FocusOn>,
        notes: Query<(), With<TutorialNote>>,
        mut commands: Commands,
    ) {
        if notes.contains(focus_on.entity) {
            commands.entity(focus_on.entity).insert(Read);
        }
    }

    /// Files each read note that the player is not looking at anymore.
    fn file_read(
        notes: Query<(Entity, &TutorialNote), With<Read>>,
        focused: Res<Focused>,
        mut run: ResMut<Run>,
        mut commands: Commands,
    ) {
        for (entity, note) in &notes {
            if focused.get() != Some(entity) {
                run.file_note(note.0);
                commands.entity(entity).despawn();
            }
        }
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
            locale.assert_formats([note.label()]);
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

    #[test]
    fn a_read_note_is_filed_once_put_down() {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.init_resource::<Focused>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.add_observer(TutorialNote::on_add);
        world.add_observer(TutorialNote::mark_read);
        world.add_observer(FocusOn::apply);
        world.init_resource::<NextState<Focus>>();
        let note = world.spawn(TutorialNote::for_step(1).unwrap()).id();
        world.flush();
        world.run_system_cached(TutorialNote::file_read).unwrap();
        assert!(world.get_entity(note).is_ok());
        world.trigger(FocusOn { entity: note });
        world.flush();
        world.run_system_cached(TutorialNote::file_read).unwrap();
        assert!(world.get_entity(note).is_ok());
        world
            .resource_mut::<Focused>()
            .leave(&mut NextState::default());
        world.run_system_cached(TutorialNote::file_read).unwrap();
        assert!(world.get_entity(note).is_err());
        assert_eq!(world.resource::<Run>().notes(), [1]);
    }
}
