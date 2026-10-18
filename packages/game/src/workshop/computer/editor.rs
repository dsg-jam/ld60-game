mod clicks;
mod footer;

use bevy::{
    input::keyboard::{Key, KeyboardInput},
    input::mouse::MouseScrollUnit,
    input_focus::{FocusCause, FocusedInput, InputFocus, tab_navigation::TabIndex},
    picking::events::PointerScroll,
    prelude::*,
    text::{EditableText, EditableTextSystems, TextCursorStyle, TextEdit},
    ui_widgets::TextInput,
};

use self::clicks::Clicks;
use self::footer::{Command, Run, Status};
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;

pub(super) const CANVAS_SIZE: UVec2 = UVec2::new(640, 480);

const PHOSPHOR: Color = Color::srgb(0.45, 1.0, 0.55);
const PHOSPHOR_DIM: Color = Color::srgb(0.2, 0.55, 0.26);
const FONT_SIZE: f32 = 13.0;
/// Keeps text clear of the bezel, which covers the edges of the glass.
const PADDING: f32 = 32.0;
const ROW_GAP: f32 = 6.0;
const INDENT: &str = "    ";
/// Lines scrolled per mouse wheel notch.
const SCROLL_LINES: f32 = 3.0;
/// Matches the default line height of 1.2 times the font size.
const LINE_HEIGHT: f32 = FONT_SIZE * 1.2;

pub(super) struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            Clicks::correct_edits.before(EditableTextSystems),
        )
        .add_observer(Editor::indent_on_tab)
        .add_observer(Command::run_on_key)
        .add_observer(Command::run_on_click)
        .add_observer(Command::keep_editor_focus)
        .add_observer(Run::apply)
        .add_observer(Editor::scroll_on_wheel);
        #[cfg(feature = "debug_port")]
        app.add_debug_method("ld60.editor", Editor::debug_state);
    }
}

/// The firmware source editor on the computer screen.
#[derive(Component)]
pub(super) struct Editor;

impl Editor {
    /// Spawns the editor UI with `source` into the canvas rendered by
    /// `camera`.
    pub(super) fn spawn(commands: &mut Commands, camera: Entity, source: &str) {
        let font = TextFont {
            font_size: FontSize::Px(FONT_SIZE),
            ..default()
        };
        let title = |text| (Text::new(text), font.clone(), TextColor(Color::BLACK));
        let mut source = EditableText {
            visible_lines: None,
            allow_newlines: true,
            ..EditableText::new(source)
        };
        source.queue_edit(TextEdit::TextStart(false));
        commands.spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                padding: px(PADDING).all(),
                row_gap: px(ROW_GAP),
                ..default()
            },
            UiTargetCamera(camera),
            children![
                (
                    Node {
                        justify_content: JustifyContent::SpaceBetween,
                        padding: px(2).horizontal(),
                        ..default()
                    },
                    BackgroundColor(PHOSPHOR),
                    children![title("FIRMWARE EDITOR"), title("CALCO REV A"),],
                ),
                (
                    Editor,
                    Name::new("editor"),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    TextInput,
                    source,
                    TextLayout::no_wrap(),
                    font.clone(),
                    TextColor(PHOSPHOR),
                    TextCursorStyle {
                        color: PHOSPHOR,
                        selection_color: PHOSPHOR_DIM,
                        ..default()
                    },
                    TabIndex(0),
                ),
                (Status, Text::default(), font.clone(), TextColor(PHOSPHOR)),
                (
                    Node {
                        column_gap: px(FONT_SIZE * 2.0),
                        ..default()
                    },
                    Children::spawn(SpawnIter(Command::ALL.into_iter().map(move |command| {
                        (
                            command,
                            Text::new(command.label(true)),
                            font.clone(),
                            TextColor(PHOSPHOR_DIM),
                        )
                    }))),
                ),
            ],
        ));
    }

    pub(super) fn take_focus(editor: Single<Entity, With<Editor>>, mut focus: ResMut<InputFocus>) {
        focus.set(*editor, FocusCause::Navigated);
    }

    pub(super) fn release_focus(mut focus: ResMut<InputFocus>) {
        focus.clear();
    }

    /// Inserts spaces instead of moving focus.
    fn indent_on_tab(
        mut input: On<FocusedInput<KeyboardInput>>,
        mut editors: Query<&mut EditableText, With<Editor>>,
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

    /// `TextInput` only scrolls to follow the caret, so the wheel is ours.
    /// Layout clamps the offset to the text.
    fn scroll_on_wheel(
        mut scroll: On<PointerScroll>,
        mut editors: Query<&mut EditableText, With<Editor>>,
    ) {
        let Ok(mut text) = editors.get_mut(scroll.entity) else {
            return;
        };
        scroll.propagate(false);
        let pixels = match scroll.unit {
            MouseScrollUnit::Line => scroll.y * SCROLL_LINES * LINE_HEIGHT,
            MouseScrollUnit::Pixel => scroll.y,
        };
        text.viewport.offset.y = (text.viewport.offset.y - pixels).max(0.0);
    }

    /// `ld60.editor` returns the text, the caret and the selected byte range.
    #[cfg(feature = "debug_port")]
    fn debug_state(
        In(_): In<Option<serde_json::Value>>,
        editor: Single<&EditableText, With<Editor>>,
    ) -> bevy::remote::BrpResult {
        let selection = editor.editor().raw_selection();
        let range = selection.text_range();
        Ok(serde_json::json!({
            "text": editor.value().to_string(),
            "caret": selection.focus().index(),
            "selection": [range.start, range.end],
        }))
    }
}
