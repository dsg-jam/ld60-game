use bevy::{
    input::keyboard::{Key, KeyboardInput},
    input_focus::FocusedInput,
    prelude::*,
    text::{EditableText, FontCx, LayoutCx, TextEdit, TextLineYBounds, cursor_reveal_rect},
};

use super::history::{History, Snapshot};

/// Undo and redo for an `EditableText`.
///
/// Any change to the text becomes an undo step, no matter where it comes
/// from. Use [`Undo::replace`] to swap out all of the text as one step.
#[derive(Component, Debug, Default)]
pub(crate) struct Undo {
    history: History,
    requests: Vec<Request>,
    reset: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Request {
    Undo,
    Redo,
}

impl Undo {
    /// Queues `source` as the new text. Undo brings back the old text.
    pub(crate) fn replace(&mut self, text: &mut EditableText, source: &str) {
        self.history.seal();
        text.queue_edit(TextEdit::SelectAll);
        text.queue_edit(TextEdit::Insert(source.into()));
        text.queue_edit(TextEdit::TextStart(false));
    }

    /// Forgets all steps. The text after the queued edits is the new start.
    pub(crate) fn reset(&mut self) {
        self.reset = true;
        self.requests.clear();
    }

    pub(crate) fn request(&mut self, request: Request) {
        self.requests.push(request);
    }

    /// Ctrl+Z undoes. Ctrl+Y and Ctrl+Shift+Z redo. Cmd works as Ctrl.
    pub(super) fn on_key(
        mut input: On<FocusedInput<KeyboardInput>>,
        keys: Res<ButtonInput<Key>>,
        mut editors: Query<&mut Undo>,
    ) {
        let Ok(mut undo) = editors.get_mut(input.focused_entity) else {
            return;
        };
        if !keys.any_pressed([Key::Control, Key::Super]) || keys.pressed(Key::Alt) {
            return;
        }
        let shift = keys.pressed(Key::Shift);
        let request = match shortcut(&input.input) {
            Some(KeyCode::KeyZ) if shift => Request::Redo,
            Some(KeyCode::KeyZ) => Request::Undo,
            Some(KeyCode::KeyY) if !shift => Request::Redo,
            _ => return,
        };
        input.propagate(false);
        if input.input.state.is_pressed() {
            undo.request(request);
        }
    }

    /// Records the edits of this frame, then undoes or redoes.
    ///
    /// Runs after `EditableText` applies its edits.
    pub(super) fn sync(
        time: Res<Time<Real>>,
        mut editors: Query<(&mut Undo, &mut EditableText)>,
        mut font_cx: ResMut<FontCx>,
        mut layout_cx: ResMut<LayoutCx>,
    ) {
        for (mut undo, mut text) in &mut editors {
            let undo = &mut *undo;
            if !text.is_changed() && !undo.reset && undo.requests.is_empty() {
                continue;
            }
            let snapshot = Snapshot::of(&text);
            if undo.reset {
                *undo = Undo {
                    history: History::new(snapshot),
                    ..default()
                };
                continue;
            }
            undo.history.observe(snapshot, time.elapsed());
            for request in undo.requests.drain(..) {
                let target = match request {
                    Request::Undo => undo.history.undo(),
                    Request::Redo => undo.history.redo(),
                };
                if let Some(target) = target {
                    restore(&mut text, target, &mut font_cx, &mut layout_cx);
                }
            }
        }
    }
}

impl Snapshot {
    fn of(text: &EditableText) -> Self {
        let selection = text.editor().raw_selection();
        Self {
            text: text.value().to_string(),
            selection: (selection.anchor().index(), selection.focus().index()),
        }
    }
}

/// The letter of a shortcut, by its meaning on the keyboard layout where it
/// is a Latin letter and by its position elsewhere.
fn shortcut(input: &KeyboardInput) -> Option<KeyCode> {
    match &input.logical_key {
        Key::Character(c) if c.eq_ignore_ascii_case("z") => Some(KeyCode::KeyZ),
        Key::Character(c) if c.eq_ignore_ascii_case("y") => Some(KeyCode::KeyY),
        Key::Character(c) if !c.is_ascii() => Some(input.key_code),
        _ => None,
    }
}

/// Shows `snapshot` in `text` and scrolls to its caret.
fn restore(
    text: &mut EditableText,
    snapshot: &Snapshot,
    font_cx: &mut FontCx,
    layout_cx: &mut LayoutCx,
) {
    let EditableText {
        editor,
        viewport,
        cursor_margin,
        ..
    } = text;
    editor.set_text(&snapshot.text);
    let mut driver = editor.driver(&mut font_cx.context, &mut layout_cx.0);
    let (anchor, focus) = snapshot.selection;
    driver.select_byte_range(anchor, focus);
    driver.refresh_layout();
    let Some(caret) = cursor_reveal_rect(&mut driver) else {
        return;
    };
    let layout = driver.layout();
    viewport.reveal_caret(
        caret,
        Vec2::new(layout.full_width(), layout.height()),
        *cursor_margin,
        layout.lines().map(|line| TextLineYBounds::from_line(&line)),
    );
}
