mod changes;
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

use self::changes::LineChanges;
use self::list::{MAX_VERSIONS, Removal, SaveError, VersionList};
use super::footer::{Flashed, Status};
use super::page::{ActivePage, Page};
use super::undo::Undo;
use super::{Editor, FONT_SIZE, PHOSPHOR, PHOSPHOR_DIM, ROW_GAP};
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::locale::{Locale, Localized, Message};
use crate::workshop::job::Job;

/// Width of the label column, so the marks line up.
const LABEL_WIDTH: usize = 26;

pub(super) struct VersionsPlugin;

impl Plugin for VersionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Versions>()
            .add_systems(
                Update,
                (
                    Versions::load_job,
                    Versions::disarm.run_if(resource_changed::<ActivePage>),
                    Versions::render
                        .run_if(resource_changed::<Versions>.or_eager(resource_changed::<Locale>)),
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
    /// When the job came on the bench, in real time.
    started: Duration,
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
    fn sync(&mut self, job: Option<(usize, &str)>, now: Duration) -> bool {
        let index = job.map(|(index, _)| index);
        if index == self.job {
            return false;
        }
        self.job = index;
        self.started = now;
        self.list = job.map(|(_, factory)| VersionList::new(factory));
        true
    }

    fn load_job(job: Option<Res<Job>>, time: Res<Time<Real>>, mut versions: ResMut<Versions>) {
        let job = job.as_ref().map(|job| (job.index, job.buggy.as_str()));
        if versions.bypass_change_detection().sync(job, time.elapsed()) {
            versions.set_changed();
        }
    }

    /// A removal waits for its second press only while the page stays open.
    fn disarm(mut versions: ResMut<Versions>) {
        if let Some(list) = versions.bypass_change_detection().list.as_mut() {
            list.disarm();
        }
    }

    fn render(
        versions: Res<Versions>,
        locale: Res<Locale>,
        mut rows: Query<(
            &VersionRow,
            &mut Node,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        )>,
    ) {
        for (version_row, mut node, mut text, mut color, mut background) in &mut rows {
            let Some((list, version)) = versions
                .list
                .as_ref()
                .and_then(|list| Some((list, list.versions().get(version_row.0)?)))
            else {
                node.display = Display::None;
                continue;
            };
            node.display = Display::Flex;
            let text_of = |message: Message| locale.text(&message.into());
            let auto = if version.auto {
                text_of(Message::new("editor-version-auto"))
            } else {
                String::new()
            };
            let flashed = if list.is_flashed(version) {
                text_of(Message::new("editor-version-flashed"))
            } else {
                String::new()
            };
            text.0 = VersionRow::text(
                version.number,
                &text_of(version.label.clone()),
                &auto,
                &flashed,
            );
            let selected = list.selected() == version_row.0;
            color.0 = if selected { Color::BLACK } else { PHOSPHOR };
            background.0 = if selected { PHOSPHOR } else { Color::NONE };
        }
    }

    fn mark_flashed(flashed: On<Flashed>, mut versions: ResMut<Versions>) {
        if let Some(list) = versions.list.as_mut() {
            list.mark_flashed(&flashed.0);
        }
    }

    /// The time on the job, the lines changed from the factory firmware and
    /// how the source assembles.
    fn label(job: &Job, factory: &str, source: &str, on_job: Duration) -> Message {
        let seconds = on_job.as_secs();
        let changes = LineChanges::between(factory, source);
        let assembled = match ld60_levels::assemble(&job.level, source) {
            Ok(assembly) => Message::new("editor-version-bytes").with("bytes", assembly.byte_len()),
            Err(LevelError::Assemble(errors)) => {
                Message::new("editor-version-errors").with("errors", errors.len())
            }
            Err(_) => Message::new("editor-version-error"),
        };
        Message::new("editor-version-label")
            .with("time", format!("{:02}:{:02}", seconds / 60, seconds % 60))
            .with("changes", changes.to_string())
            .with("result", assembled)
    }

    /// `ld60.versions` returns the versions of the current job and the
    /// selected position.
    #[cfg(feature = "debug_port")]
    fn debug_state(
        In(_): In<Option<serde_json::Value>>,
        versions: Res<Versions>,
        locale: Res<Locale>,
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
                    "label": locale.text(&version.label.clone().into()),
                    "flashed": list.is_flashed(version),
                    "auto": version.auto,
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
                            Localized::from(action.label()),
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
    /// The line of the version list. A long label is cut, so the marks line
    /// up.
    fn text(number: u32, label: &str, auto: &str, flashed: &str) -> String {
        let label: String = label.chars().take(LABEL_WIDTH - 1).collect();
        format!("{number:>2}  {label:<LABEL_WIDTH$}{auto:<5}{flashed}")
            .trim_end()
            .into()
    }

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

    fn label(self) -> Message {
        Message::new(match self {
            Self::Save => "editor-action-save",
            Self::Restore => "editor-action-restore",
            Self::Remove => "editor-action-remove",
        })
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
        editor: Single<(&mut EditableText, &mut Undo), With<Editor>>,
        mut status: Single<&mut Localized, With<Status>>,
        mut active: ResMut<ActivePage>,
    ) {
        let (mut editor, mut undo) = editor.into_inner();
        let on_job = time.elapsed().saturating_sub(versions.started);
        let (Some(job), Some(list)) = (job, versions.list.as_mut()) else {
            **status = Message::new("editor-no-job").into();
            return;
        };
        let message = match perform.0 {
            Action::Save => {
                let source = editor.value().to_string();
                let label = Versions::label(&job, &list.factory().text, &source, on_job);
                match list.save(&source, label) {
                    Ok(number) => Message::new("editor-saved").with("version", number),
                    Err(SaveError::Duplicate(number)) => {
                        Message::new("editor-duplicate").with("version", number)
                    }
                    Err(SaveError::Full) => Message::new("editor-full"),
                }
            }
            Action::Restore => {
                let source = editor.value().to_string();
                let label = Versions::label(&job, &list.factory().text, &source, on_job);
                let version = list.selected_version();
                let (number, text) = (version.number, version.text.clone());
                match list.keep(&source, label) {
                    Ok(kept) => {
                        editor.viewport.offset.y = 0.0;
                        undo.replace(&mut editor, &text);
                        active.0 = Page::Source;
                        match kept {
                            Some(kept) => Message::new("editor-restored-kept")
                                .with("version", number)
                                .with("kept", kept),
                            None => Message::new("editor-restored").with("version", number),
                        }
                    }
                    Err(_) => Message::new("editor-full-keep"),
                }
            }
            Action::Remove => match list.remove_selected() {
                Removal::Factory => Message::new("editor-factory-stays"),
                Removal::Armed(number) => {
                    Message::new("editor-remove-armed").with("version", number)
                }
                Removal::Removed(number) => Message::new("editor-removed").with("version", number),
            },
        };
        **status = message.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_keep_the_marks_in_line() {
        let long = VersionRow::text(12, "123:45  +100 -100  100 ERRORS", "AUTO", "FLASHED");
        let short = VersionRow::text(12, "00:05  +1 -1  42 BYTES", "", "FLASHED");
        assert_eq!(long.find("AUTO"), Some(4 + LABEL_WIDTH));
        assert_eq!(long.find("FLASHED"), short.find("FLASHED"));
        assert_eq!(short.find("FLASHED"), Some(4 + LABEL_WIDTH + 5));
    }

    #[test]
    fn a_new_job_starts_a_new_list() {
        let mut versions = Versions::default();
        assert!(versions.sync(Some((0, "first")), Duration::ZERO));
        let list = versions.list.as_mut().unwrap();
        list.save("fix", Message::new("A")).unwrap();
        assert!(!versions.sync(Some((0, "first")), Duration::ZERO));
        assert_eq!(versions.list.as_ref().unwrap().versions().len(), 2);

        assert!(versions.sync(Some((1, "second")), Duration::from_secs(60)));
        assert_eq!(versions.started, Duration::from_secs(60));
        let list = versions.list.as_ref().unwrap();
        assert_eq!(list.versions().len(), 1);
        assert_eq!(list.selected_version().text, "second");

        assert!(versions.sync(None, Duration::ZERO));
        assert!(versions.list.is_none());
    }
}
