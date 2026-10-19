use bevy::prelude::*;

use super::clock::{Busy, Chore};
use super::job::Job;
use super::run::Origin;
use crate::locale::Message;
use crate::menu::AppState;
use crate::parts::paper::Paper;
use crate::parts::screen::UiCanvas;
use crate::player::focus::{Focus, FocusOn, FocusTarget, View};
use crate::player::hints::NextStep;

/// In front of the seat, between the device and the keyboard. Relative to
/// the mat, with the whole note on it.
pub(super) const SPOT: Vec2 = Vec2::new(0.31, 0.45);
pub(super) const YAW: f32 = -0.05;
const SHEET_SIZE: Vec2 = Vec2::new(0.15, 0.2);
const CANVAS_SIZE: UVec2 = UVec2::new(450, 600);
const SHEET_GAP: f32 = 0.0005;
/// The paragraphs of each note, as attributes of its message.
const LESSONS: [&[&str]; 2] = [
    &["welcome", "ticket", "computer", "flash", "try", "finish"],
    &["datasheet", "lines", "versions", "rent", "trader"],
];

pub(crate) struct TutorialPlugin;

impl Plugin for TutorialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tried>()
            .add_observer(TutorialNote::on_add)
            .add_observer(Tried::forget)
            .add_systems(
                Update,
                (
                    TutorialNote::open_unread
                        .run_if(in_state(AppState::Playing))
                        .run_if(in_state(Focus::Desk)),
                    Step::show,
                ),
            );
    }
}

/// A note from the human who ran the shop before, left next to a tutorial
/// job. Step `0` of the tutorial gets the first note.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct TutorialNote(pub(crate) usize);

/// Marks the first note until it opens on its own.
#[derive(Component, Debug)]
struct Unread;

/// Whether the player looked at the device since the last flash.
#[derive(Resource, Default, Debug)]
struct Tried(bool);

/// The next thing to do in the first tutorial job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Fix,
    Flash,
    Try,
    Stamp,
    Ring,
}

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
        if note.0 == 0 {
            commands.entity(add.entity).insert(Unread);
        }
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

    fn open_unread(notes: Query<Entity, With<Unread>>, mut commands: Commands) {
        for note in &notes {
            commands.entity(note).remove::<Unread>();
            commands.trigger(FocusOn { entity: note });
        }
    }
}

impl Step {
    /// The next step of `job` while the player looks at `focus`, if `job` is
    /// the first of the tutorial. `tried` says whether the player looked at
    /// the device since the last flash.
    fn next(job: &Job, focus: Focus, tried: bool) -> Option<Self> {
        if !matches!(job.origin, Origin::Lesson { step: 0, .. }) {
            return None;
        }
        Some(match (job.flashed, job.stamped) {
            (_, true) => Self::Ring,
            (false, false) if focus == Focus::Computer => Self::Flash,
            (false, false) => Self::Fix,
            (true, false) if tried => Self::Stamp,
            (true, false) => Self::Try,
        })
    }

    fn message(self) -> Message {
        Message::new(match self {
            Self::Fix => "tutorial-step-fix",
            Self::Flash => "tutorial-step-flash",
            Self::Try => "tutorial-step-try",
            Self::Stamp => "tutorial-step-stamp",
            Self::Ring => "tutorial-step-ring",
        })
    }

    fn show(
        job: Option<Res<Job>>,
        focus: Res<State<Focus>>,
        mut tried: ResMut<Tried>,
        mut hint: ResMut<NextStep>,
    ) {
        let focus = *focus.get();
        let step = job.as_deref().and_then(|job| {
            if !job.flashed {
                tried.0 = false;
            } else if focus == Focus::Device {
                tried.0 = true;
            }
            Self::next(job, focus, tried.0)
        });
        hint.set_if_neq(NextStep(step.map(Self::message)));
    }
}

impl Tried {
    fn forget(busy: On<Busy>, mut tried: ResMut<Tried>) {
        if busy.0 == Chore::Flash {
            tried.0 = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::{Locale, Localized};
    use crate::workshop::levels::Levels;
    use crate::workshop::pickup::Pickup;
    use crate::workshop::run::Run;

    #[test]
    fn every_note_formats() {
        let locale = Locale::on_disk();
        for step in 0..LESSONS.len() {
            let note = TutorialNote(step);
            locale.assert_formats(Localized::spawned_by(|page| note.write(page)));
        }
        locale.assert_formats([Message::new("tutorial-footer"), Message::new("click-note")]);
        locale.assert_formats(
            [Step::Fix, Step::Flash, Step::Try, Step::Stamp, Step::Ring].map(Step::message),
        );
    }

    #[test]
    fn steps_follow_the_first_job() {
        let levels = Levels::on_disk();
        let mut job = Job::for_run(&Run::default(), &levels).unwrap();
        let next = |job: &Job, focus, tried| Step::next(job, focus, tried);
        assert_eq!(next(&job, Focus::Desk, false), Some(Step::Fix));
        assert_eq!(next(&job, Focus::Computer, false), Some(Step::Flash));
        job.flashed = true;
        assert_eq!(next(&job, Focus::Desk, false), Some(Step::Try));
        assert_eq!(next(&job, Focus::Desk, true), Some(Step::Stamp));
        job.stamped = true;
        assert_eq!(next(&job, Focus::Desk, true), Some(Step::Ring));
        let mut run = Run::default();
        run.settle(&job, &Pickup::Paid(job.level.reward));
        let second = Job::for_run(&run, &levels).unwrap();
        assert!(matches!(second.origin, Origin::Lesson { step: 1, .. }));
        assert_eq!(next(&second, Focus::Desk, false), None);
        let after = Job::for_run(&Run::default().next(), &levels).unwrap();
        assert_eq!(next(&after, Focus::Desk, false), None);
    }

    #[test]
    fn a_new_flash_wants_a_new_try() {
        let mut world = World::new();
        world.insert_resource(Tried(true));
        world.add_observer(Tried::forget);
        world.trigger(Busy(Chore::HandIn));
        assert!(world.resource::<Tried>().0);
        world.trigger(Busy(Chore::Flash));
        assert!(!world.resource::<Tried>().0);
    }

    #[test]
    fn every_tutorial_job_has_a_note() {
        let steps = Levels::on_disk().tutorial().len();
        assert!((2..=3).contains(&steps));
        assert!((0..steps).all(|step| TutorialNote::for_step(step).is_some()));
        assert!(TutorialNote::for_step(steps).is_none());
    }
}
