use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::job::Job;
use super::run::{Origin, Run};
use crate::locale::Message;
use crate::parts::paper::{Paper, Press, Print, Stock};

/// The paragraphs of each note, as attributes of its message.
const LESSONS: [&[&str]; 2] = [
    &["welcome", "ticket", "computer", "flash", "try", "finish"],
    &["datasheet", "lines", "versions", "rent", "trader"],
];

pub(crate) struct TutorialPlugin;

impl Plugin for TutorialPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            TutorialNote::file_for_job.run_if(resource_exists_and_changed::<Job>),
        );
    }
}

/// A note from the human who ran the shop before, about a step of the
/// tutorial. Step `0` of the tutorial gets the first note.
///
/// Each note is filed in the manuals file when the job of its step comes
/// in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct TutorialNote(pub(crate) usize);

/// The first note was just filed. The manuals file opens at it once the
/// player sits at the bench, as if it was left open there for them.
#[derive(Resource, Debug)]
pub(crate) struct FirstNote;

impl TutorialNote {
    /// The note of step `step`, if it has one.
    pub(super) fn new(step: usize) -> Option<Self> {
        (step < LESSONS.len()).then_some(Self(step))
    }

    /// The note about `job`, if it is a job of the tutorial with one.
    pub(crate) fn for_job(job: &Job) -> Option<Self> {
        match job.origin {
            Origin::Lesson { step, .. } => Self::new(step),
            _ => None,
        }
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

    /// Marta typed her notes.
    pub(super) fn print(self) -> Print {
        Press::Typewriter
            .run(("tutorial", self.0))
            .on(Stock::TYPING)
    }

    pub(super) fn write(self, page: &mut ChildSpawnerCommands) {
        let message = self.message();
        page.spawn(Paper::heading(self.title()));
        for &paragraph in LESSONS[self.0] {
            page.spawn(Paper::text(message.clone().attribute(paragraph)));
        }
    }

    /// Files the note about the job that came in, unless it is filed
    /// already.
    fn file_for_job(job: Res<Job>, mut run: ResMut<Run>, mut commands: Commands) {
        let Some(note) = Self::for_job(&job) else {
            return;
        };
        if run.notes().contains(&note.0) {
            return;
        }
        run.file_note(note.0);
        if note.0 == 0 {
            commands.insert_resource(FirstNote);
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
        locale.assert_formats([Message::new("tutorial-footer")]);
    }

    #[test]
    fn every_tutorial_job_has_a_note() {
        let steps = Levels::on_disk().tutorial().len();
        assert!((2..=3).contains(&steps));
        assert!((0..steps).all(|step| TutorialNote::new(step).is_some()));
        assert!(TutorialNote::new(steps).is_none());
    }

    #[test]
    fn a_tutorial_job_files_its_note_and_the_first_one_opens() {
        let levels = Levels::on_disk();
        let mut world = World::new();
        world.init_resource::<Run>();
        let lesson = |step: usize| {
            let mut job = Job::load(&levels, 0).unwrap().unwrap();
            job.origin = Origin::Lesson {
                step,
                source: levels.levels()[0].clone(),
            };
            job
        };
        for step in 0..2 {
            world.insert_resource(lesson(step));
            world.run_system_cached(TutorialNote::file_for_job).unwrap();
            assert_eq!(world.contains_resource::<FirstNote>(), step == 0);
            world.remove_resource::<FirstNote>();
        }
        assert_eq!(world.resource::<Run>().notes(), [0, 1]);
        world.insert_resource(lesson(0));
        world.run_system_cached(TutorialNote::file_for_job).unwrap();
        assert!(!world.contains_resource::<FirstNote>());
    }
}
