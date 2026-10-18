mod list;

use std::time::Duration;

use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{FocusedInput, tab_navigation::TabIndex},
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    text::EditableText,
};
use ld60_levels::LevelError;

use self::list::{MAX_VERSIONS, SaveError, VersionList};
use super::footer::{Flashed, Status};
use super::page::{ActivePage, Page};
use super::{Editor, FONT_SIZE, PHOSPHOR, PHOSPHOR_DIM, ROW_GAP};
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::workshop::job::Job;

/// Width of the label column, so the flash mark lines up.
const LABEL_WIDTH: usize = 20;

pub(super) struct VersionsPlugin;

impl Plugin for VersionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Versions>()
            .add_systems(
                Update,
                (
                    Versions::load_job,
                    Versions::render.run_if(resource_changed::<Versions>),
                )
                    .chain(),
            )
            .add_observer(Versions::mark_flashed)
            .add_observer(VersionRow::select_on_click)
            .add_observer(Action::run_on_key)
            .add_observer(Action::run_on_click)
            .add_observer(Perform::apply);
        #[cfg(feature = "debug_port")]
        app.add_debug_method("ld60.versions", Versions::debug_state);
    }
}

/// The saved versions of the current job's source. A new job starts a new
/// list.
#[derive(Resource, Debug, Default)]
struct Versions {
    /// Position of the job in the catalog.
    job: Option<usize>,
    list: Option<VersionList>,
}

/// The page that lists the [`Versions`].
#[derive(Component, Debug)]
pub(super) struct VersionsPage;

/// A line of the version list, by position.
#[derive(Component, Debug)]
struct VersionRow(usize);

/// Something to do with the version list. Each one runs by clicking it or
/// with its key.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Save,
    Restore,
    Remove,
}

/// Runs an [`Action`].
#[derive(Event, Debug)]
struct Perform(Action);

impl Versions {
    /// Starts a new list when the job changes. Returns whether it did.
    fn sync(&mut self, job: Option<(usize, &str)>) -> bool {
        let index = job.map(|(index, _)| index);
        if index == self.job {
            return false;
        }
        self.job = index;
        self.list = job.map(|(_, factory)| VersionList::new(factory));
        true
    }

    fn load_job(job: Option<Res<Job>>, mut versions: ResMut<Versions>) {
        let job = job.as_ref().map(|job| (job.index, job.buggy.as_str()));
        if versions.bypass_change_detection().sync(job) {
            versions.set_changed();
        }
    }

    fn render(
        versions: Res<Versions>,
        mut rows: Query<(
            &VersionRow,
            &mut Node,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        )>,
    ) {
        for (row, mut node, mut text, mut color, mut background) in &mut rows {
            let Some((list, version)) = versions
                .list
                .as_ref()
                .and_then(|list| Some((list, list.versions().get(row.0)?)))
            else {
                node.display = Display::None;
                continue;
            };
            node.display = Display::Flex;
            let flashed = if list.is_flashed(version) {
                "FLASHED"
            } else {
                ""
            };
            text.0 = format!(
                "{:>2}  {:<LABEL_WIDTH$}{flashed}",
                version.number, version.label
            );
            let selected = list.selected() == row.0;
            color.0 = if selected { Color::BLACK } else { PHOSPHOR };
            background.0 = if selected { PHOSPHOR } else { Color::NONE };
        }
    }

    fn mark_flashed(flashed: On<Flashed>, mut versions: ResMut<Versions>) {
        if let Some(list) = versions.list.as_mut() {
            list.mark_flashed(&flashed.0);
        }
    }

    /// The time of saving and how the source assembles.
    fn label(job: &Job, source: &str, time: Duration) -> String {
        let seconds = time.as_secs();
        let assembled = match ld60_levels::assemble(&job.level, source) {
            Ok(assembly) => {
                let bytes: u16 = assembly.lines.iter().map(|line| line.len).sum();
                format!("{bytes} BYTES")
            }
            Err(LevelError::Assemble(errors)) if errors.len() == 1 => "1 ERROR".into(),
            Err(LevelError::Assemble(errors)) => format!("{} ERRORS", errors.len()),
            Err(_) => "ERROR".into(),
        };
        format!("{:02}:{:02}  {assembled}", seconds / 60, seconds % 60)
    }

    /// `ld60.versions` returns the versions of the current job and the
    /// selected position.
    #[cfg(feature = "debug_port")]
    fn debug_state(
        In(_): In<Option<serde_json::Value>>,
        versions: Res<Versions>,
    ) -> bevy::remote::BrpResult {
        let Some(list) = &versions.list else {
            return Ok(serde_json::Value::Null);
        };
        let versions: Vec<_> = list
            .versions()
            .iter()
            .map(|version| {
                serde_json::json!({
                    "number": version.number,
                    "label": version.label,
                    "flashed": list.is_flashed(version),
                    "text": version.text,
                })
            })
            .collect();
        Ok(serde_json::json!({
            "versions": versions,
            "selected": list.selected(),
        }))
    }
}

impl VersionsPage {
    /// The versions page, hidden until the player opens it.
    pub(super) fn bundle(font: TextFont) -> impl Bundle {
        let row_font = font.clone();
        (
            VersionsPage,
            Page::Versions,
            Name::new("versions"),
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(ROW_GAP),
                display: Display::None,
                ..default()
            },
            TabIndex(0),
            children![
                (
                    Node {
                        column_gap: px(FONT_SIZE * 2.0),
                        ..default()
                    },
                    Children::spawn(SpawnIter(Action::ALL.into_iter().map(move |action| {
                        (
                            action,
                            Text::new(action.label()),
                            font.clone(),
                            TextColor(PHOSPHOR_DIM),
                        )
                    }))),
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    Children::spawn(SpawnIter((0..MAX_VERSIONS).map(move |index| {
                        (
                            VersionRow(index),
                            Node {
                                display: Display::None,
                                padding: px(2).horizontal(),
                                ..default()
                            },
                            Text::default(),
                            row_font.clone(),
                            TextColor(PHOSPHOR),
                            BackgroundColor(Color::NONE),
                        )
                    }))),
                ),
            ],
        )
    }
}

impl VersionRow {
    fn select_on_click(
        click: On<PointerClick>,
        rows: Query<&VersionRow>,
        mut versions: ResMut<Versions>,
    ) {
        if click.button == PointerButton::Primary
            && let Ok(row) = rows.get(click.entity)
            && let Some(list) = versions.list.as_mut()
        {
            list.select(row.0);
        }
    }
}

impl Action {
    const ALL: [Self; 3] = [Self::Save, Self::Restore, Self::Remove];

    fn label(self) -> &'static str {
        match self {
            Self::Save => "S SAVE",
            Self::Restore => "ENTER RESTORE",
            Self::Remove => "DEL REMOVE",
        }
    }

    /// Arrows move the selection. The page takes all other keys too, so
    /// none of them reach tab navigation.
    fn run_on_key(
        mut input: On<FocusedInput<KeyboardInput>>,
        pages: Query<(), With<VersionsPage>>,
        mut versions: ResMut<Versions>,
        mut commands: Commands,
    ) {
        if !pages.contains(input.focused_entity) {
            return;
        }
        input.propagate(false);
        if !input.input.state.is_pressed() {
            return;
        }
        let action = match input.input.key_code {
            KeyCode::KeyS if !input.input.repeat => Self::Save,
            KeyCode::Enter | KeyCode::NumpadEnter if !input.input.repeat => Self::Restore,
            KeyCode::Delete | KeyCode::Backspace if !input.input.repeat => Self::Remove,
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                let step = if input.input.key_code == KeyCode::ArrowUp {
                    -1
                } else {
                    1
                };
                if let Some(list) = versions.list.as_mut() {
                    list.select_by(step);
                }
                return;
            }
            _ => return,
        };
        commands.trigger(Perform(action));
    }

    fn run_on_click(click: On<PointerClick>, actions: Query<&Action>, mut commands: Commands) {
        if click.button == PointerButton::Primary
            && let Ok(&action) = actions.get(click.entity)
        {
            commands.trigger(Perform(action));
        }
    }
}

impl Perform {
    fn apply(
        perform: On<Perform>,
        job: Option<Res<Job>>,
        time: Res<Time<Real>>,
        mut versions: ResMut<Versions>,
        mut editor: Single<&mut EditableText, With<Editor>>,
        mut status: Single<&mut Text, With<Status>>,
        mut active: ResMut<ActivePage>,
    ) {
        let (Some(job), Some(list)) = (job, versions.list.as_mut()) else {
            status.0 = "NO JOB ON THE BENCH".into();
            return;
        };
        status.0 = match perform.0 {
            Action::Save => {
                let source = editor.value().to_string();
                let label = Versions::label(&job, &source, time.elapsed());
                match list.save(&source, label) {
                    Ok(number) => format!("SAVED AS VERSION {number}"),
                    Err(SaveError::Duplicate(number)) => format!("SAME AS VERSION {number}"),
                    Err(SaveError::Full) => "NO ROOM. REMOVE A VERSION FIRST.".into(),
                }
            }
            Action::Restore => {
                let version = list.selected_version();
                Editor::replace_text(&mut editor, &version.text);
                active.0 = Page::Source;
                format!("RESTORED VERSION {}", version.number)
            }
            Action::Remove => match list.remove_selected() {
                Some(number) => format!("REMOVED VERSION {number}"),
                None => "THE FACTORY VERSION STAYS".into(),
            },
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_job_starts_a_new_list() {
        let mut versions = Versions::default();
        assert!(versions.sync(Some((0, "first"))));
        let list = versions.list.as_mut().unwrap();
        list.save("fix", "A".into()).unwrap();
        assert!(!versions.sync(Some((0, "first"))));
        assert_eq!(versions.list.as_ref().unwrap().versions().len(), 2);

        assert!(versions.sync(Some((1, "second"))));
        let list = versions.list.as_ref().unwrap();
        assert_eq!(list.versions().len(), 1);
        assert_eq!(list.selected_version().text, "second");

        assert!(versions.sync(None));
        assert!(versions.list.is_none());
    }
}
