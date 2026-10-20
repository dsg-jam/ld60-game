mod bench;
mod breakpoints;
mod chip_title;
mod clicks;
mod debugger;
mod edits;
mod errors;
mod footer;
mod highlight;
mod history;
mod lexer;
mod listing;
mod notes;
mod page;
mod shades;
mod trace;
pub(super) mod undo;
mod versions;

use bevy::{
    input::keyboard::{Key, KeyboardInput},
    input::mouse::MouseScrollUnit,
    input_focus::{FocusedInput, InputFocus, tab_navigation::TabIndex},
    picking::events::PointerScroll,
    prelude::*,
    text::{EditableText, TextCursorStyle, TextEdit, detect_text_needs_rerender},
    ui::{UiSystems, widget::update_editable_text_styles},
    ui_widgets::TextInput,
};

use self::bench::Verdict;
use self::breakpoints::Breakpoints;
use self::chip_title::ChipTitle;
use self::clicks::Clicks;
use self::debugger::DebugPanel;
use self::footer::{Command, Run, Status};
use self::highlight::Highlight;
use self::listing::Listing;
use self::notes::Notes;
use self::page::{Page, PageTitle};
use self::shades::Shades;
use self::trace::TraceView;
use self::undo::Undo;
use self::versions::VersionsPage;
use super::views::ZOOM_MODIFIERS;
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::locale::{Localized, Message};
use crate::parts::screen::UiCanvas;
use crate::workshop::job::{Job, JobId};

pub(super) const CANVAS_SIZE: UVec2 = UVec2::new(640, 480);

const PHOSPHOR: Color = Color::srgb(0.45, 1.0, 0.55);
const PHOSPHOR_DIM: Color = Color::srgb(0.2, 0.55, 0.26);
/// Dark enough for the dimmest highlight color to stay readable on it.
const SELECTION: Color = Color::srgb(0.1, 0.3, 0.14);
const FONT_SIZE: f32 = 13.0;
/// Keeps text clear of the bezel, which covers the edges of the glass.
const PADDING: f32 = 32.0;
const ROW_GAP: f32 = 6.0;
const INDENT: &str = "    ";
/// Lines scrolled per mouse wheel notch.
const SCROLL_LINES: f32 = 3.0;
/// Matches the default line height of 1.2 times the font size.
const LINE_HEIGHT: f32 = FONT_SIZE * 1.2;
/// Room left of the source for clicking breakpoints.
const MARGIN: f32 = FONT_SIZE;

pub(super) struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                (
                    Clicks::correct_edits,
                    Editor::apply_edits.after(update_editable_text_styles),
                    Undo::sync,
                    Highlight::refresh,
                )
                    .chain()
                    .before(detect_text_needs_rerender),
                Highlight::place.after(UiSystems::PostLayout),
            ),
        )
        .init_resource::<Listing>()
        .init_resource::<Breakpoints>()
        .init_resource::<TraceView>()
        .init_resource::<Shades>()
        .add_systems(
            Update,
            (
                Editor::load_job,
                Command::show_labels,
                Command::show_owned,
                (
                    (Listing::follow_job, Breakpoints::follow_job),
                    Breakpoints::toggle_on_margin,
                    Breakpoints::arm,
                    (Shades::update, DebugPanel::update),
                )
                    .chain(),
            ),
        )
        .add_systems(
            Update,
            ChipTitle::update.run_if(resource_changed_or_removed::<Job>),
        )
        .add_observer(Editor::indent_on_tab)
        .add_observer(Undo::on_key)
        .add_observer(Command::run_on_key)
        .add_observer(Command::run_on_click)
        .add_observer(Command::keep_editor_focus)
        .add_observer(Run::apply)
        .add_observer(Verdict::run_on_command)
        .add_observer(DebugPanel::run_on_command)
        .add_observer(Breakpoints::toggle_on_run)
        .add_observer(TraceView::toggle_on_run)
        .add_observer(Listing::follow_flash)
        .add_observer(Editor::scroll_on_wheel);
        app.add_plugins((page::PagePlugin, versions::VersionsPlugin));
        #[cfg(feature = "debug_port")]
        app.add_debug_method("ld60.editor", Editor::debug_state);
        #[cfg(feature = "debug_port")]
        app.add_debug_method("ld60.notes", Notes::debug_state);
    }
}

/// The firmware source editor on the computer screen.
#[derive(Component)]
pub(super) struct Editor;

impl Editor {
    /// Spawns the editor UI into `canvas`. The firmware of the job comes
    /// with [`Editor::load_job`].
    pub(super) fn spawn(commands: &mut Commands, canvas: &UiCanvas) {
        let font = TextFont {
            font_size: FontSize::Px(FONT_SIZE),
            ..default()
        };
        let title = |text: Localized| (text, font.clone(), TextColor(Color::BLACK));
        let source = EditableText {
            visible_lines: None,
            allow_newlines: true,
            ..default()
        };
        commands.spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                padding: px(PADDING).all(),
                row_gap: px(ROW_GAP),
                ..default()
            },
            canvas.target(),
            children![
                (
                    Node {
                        justify_content: JustifyContent::SpaceBetween,
                        padding: px(2).horizontal(),
                        ..default()
                    },
                    BackgroundColor(PHOSPHOR),
                    children![
                        (PageTitle, title(Page::Source.title().into())),
                        (title(Localized::verbatim(ChipTitle::NONE)), ChipTitle)
                    ],
                ),
                (
                    Editor,
                    Page::Source,
                    Name::new("editor"),
                    Node {
                        flex_grow: 1.0,
                        overflow: Overflow::clip(),
                        padding: UiRect::left(px(MARGIN)),
                        ..default()
                    },
                    TextInput,
                    source,
                    TextLayout::no_wrap(),
                    font.clone(),
                    // `Highlight` draws the text.
                    TextColor(Color::NONE),
                    TextCursorStyle {
                        color: PHOSPHOR,
                        selection_color: SELECTION,
                        ..default()
                    },
                    TabIndex(0),
                    Undo::default(),
                    children![Highlight::bundle(font.clone())],
                ),
                Notes::bundle(font.clone()),
                VersionsPage::bundle(font.clone()),
                DebugPanel::bundle(font.clone()),
                (
                    Status,
                    Localized::verbatim(""),
                    font.clone(),
                    TextColor(PHOSPHOR)
                ),
                (
                    Node {
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(FONT_SIZE * 1.5),
                        row_gap: px(ROW_GAP),
                        ..default()
                    },
                    // `Command::show_labels` fills in the labels and
                    // `Command::show_owned` shows the ones of owned tools.
                    Children::spawn(SpawnIter(Command::ALL.into_iter().map(move |command| {
                        (
                            command,
                            Node {
                                display: Display::None,
                                ..default()
                            },
                            Localized::verbatim(""),
                            font.clone(),
                            TextColor(PHOSPHOR_DIM),
                        )
                    }))),
                ),
            ],
        ));
    }

    /// Puts the firmware of a new job into the editor.
    fn load_job(
        job: Option<Res<Job>>,
        mut loaded: Local<Option<JobId>>,
        editor: Single<(&mut EditableText, &mut Undo), With<Editor>>,
        mut status: Single<&mut Localized, With<Status>>,
    ) {
        let id = job.as_ref().map(|job| job.id);
        if id == *loaded {
            return;
        }
        *loaded = id;
        let (mut text, mut undo) = editor.into_inner();
        text.clear();
        text.viewport.offset.y = 0.0;
        match job {
            Some(job) => {
                undo.replace(&mut text, &job.buggy);
                **status = Localized::verbatim("");
            }
            None => **status = Message::new("editor-no-more-jobs").into(),
        }
        undo.reset();
    }

    pub(super) fn release_focus(mut focus: ResMut<InputFocus>) {
        focus.clear();
    }

    /// Inserts spaces instead of moving focus.
    fn indent_on_tab(
        mut input: On<FocusedInput<KeyboardInput>>,
        mut editors: Query<&mut EditableText, With<Page>>,
    ) {
        let Ok(mut text) = editors.get_mut(input.focused_entity) else {
            return;
        };
        if input.input.logical_key == Key::Tab {
            input.propagate(false);
            if input.input.state.is_pressed() {
                text.queue_edit(TextEdit::Insert(INDENT.into()));
            }
        }
    }

    /// `TextInput` only scrolls to follow the caret, so the wheel is ours,
    /// unless it zooms. Layout clamps the offset to the text.
    fn scroll_on_wheel(
        mut scroll: On<PointerScroll>,
        keys: Res<ButtonInput<KeyCode>>,
        mut editors: Query<&mut EditableText, With<Page>>,
    ) {
        let Ok(mut text) = editors.get_mut(scroll.entity) else {
            return;
        };
        if keys.any_pressed(ZOOM_MODIFIERS) {
            return;
        }
        scroll.propagate(false);
        let pixels = match scroll.unit {
            MouseScrollUnit::Line => scroll.y * SCROLL_LINES * LINE_HEIGHT,
            MouseScrollUnit::Pixel => scroll.y,
        };
        text.viewport.offset.y = (text.viewport.offset.y - pixels).max(0.0);
    }

    /// `ld60.editor { "history": "undo" }` returns the text, the caret, the
    /// selected byte range and the assembler errors.
    ///
    /// The optional `history` is `undo` or `redo`. It applies in the next
    /// frame.
    #[cfg(feature = "debug_port")]
    fn debug_state(
        In(params): In<Option<serde_json::Value>>,
        editor: Single<(&EditableText, &mut Undo), With<Editor>>,
        job: Option<Res<Job>>,
    ) -> bevy::remote::BrpResult {
        use serde_json::{Value, json};

        let (text, mut undo) = editor.into_inner();
        let step = params.as_ref().and_then(|params| params.get("history"));
        match step.map(Value::as_str) {
            None => {}
            Some(Some("undo")) => undo.request(self::undo::Request::Undo),
            Some(Some("redo")) => undo.request(self::undo::Request::Redo),
            Some(_) => {
                return Err(bevy::remote::BrpError {
                    code: bevy::remote::error_codes::INVALID_PARAMS,
                    message: "`history` must be \"undo\" or \"redo\"".into(),
                    data: None,
                });
            }
        }
        let source = text.value().to_string();
        let errors = job
            .and_then(|job| job.isa().assemble(&source).err())
            .unwrap_or_default();
        let selection = text.editor().raw_selection();
        let range = selection.text_range();
        Ok(json!({
            "caret": selection.focus().index(),
            "selection": [range.start, range.end],
            "errors": errors
                .iter()
                .map(|error| json!({
                    "line": error.line,
                    "span": [error.span.start, error.span.end],
                    "message": error.kind.to_string(),
                }))
                .collect::<Vec<_>>(),
            "text": source,
        }))
    }
}
