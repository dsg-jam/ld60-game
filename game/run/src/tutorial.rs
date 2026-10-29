use bevy::prelude::*;
use ld60_locale::Message;
use serde::{Deserialize, Serialize};

use crate::job::Job;
use crate::{Origin, Run};

/// The paragraphs of each note, as attributes of its message.
const LESSONS: [&[&str]; 7] = [
    &[
        "welcome", "ticket", "open", "cable", "computer", "fix", "flash", "finish", "idea",
    ],
    &["idea", "inc"],
    &["idea", "jump", "wait"],
    &["idea", "manual", "datasheet"],
    &["idea", "jump"],
    &["idea", "digit"],
    &["versions", "rent", "trader"],
];

pub struct TutorialPlugin;

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
pub struct TutorialNote(pub(crate) usize);

/// The first note was just pinned to the monitor. The player looks at it
/// once they sit at the bench, as if it was left there for them.
#[derive(Resource, Debug)]
pub struct FirstNote;

impl TutorialNote {
    pub const FIRST: Self = Self(0);

    /// The note of step `step`, if it has one.
    pub fn new(step: usize) -> Option<Self> {
        (step < LESSONS.len()).then_some(Self(step))
    }

    pub fn step(self) -> usize {
        self.0
    }

    /// The note about `job`, if it is a job of the tutorial with one.
    pub fn for_job(job: &Job) -> Option<Self> {
        match job.origin {
            Origin::Lesson { step, .. } => Self::new(step),
            _ => None,
        }
    }

    fn message(self) -> Message {
        Message::new(format!("tutorial-{}", self.0 + 1))
    }

    pub fn title(self) -> Message {
        self.message().attribute("title")
    }

    /// The short name of the note in the index of the manuals file.
    pub fn label(self) -> Message {
        self.message().attribute("label")
    }

    pub fn footer(self) -> Message {
        Message::new("tutorial-footer")
    }

    /// The notes of the steps before `step`.
    pub(crate) fn before(step: usize) -> impl Iterator<Item = Self> {
        (0..step.min(LESSONS.len())).map(Self)
    }

    pub fn paragraphs(self) -> impl Iterator<Item = Message> {
        let message = self.message();
        LESSONS[self.0]
            .iter()
            .map(move |&paragraph| message.clone().attribute(paragraph))
    }

    /// Files the note about the job that came in, unless it is filed
    /// already, with the manual of its gadget.
    fn file_for_job(job: Res<Job>, mut run: ResMut<Run>, mut commands: Commands) {
        let Some(note) = Self::for_job(&job) else {
            return;
        };
        run.file_manual(job.level.device);
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
    use ld60_locale::Locale;

    use super::*;
    use crate::levels::Levels;

    #[test]
    fn every_note_formats() {
        let locale = Locale::on_disk();
        for step in 0..LESSONS.len() {
            let note = TutorialNote(step);
            locale.assert_formats(note.paragraphs());
            locale.assert_formats([note.title(), note.label()]);
        }
        locale.assert_formats([Message::new("tutorial-footer")]);
    }

    #[test]
    fn every_tutorial_job_has_a_note() {
        let steps = Levels::on_disk().tutorial().len();
        assert_eq!(steps, LESSONS.len());
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
