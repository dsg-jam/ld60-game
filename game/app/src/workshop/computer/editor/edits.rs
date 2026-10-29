use bevy::{
    clipboard::Clipboard,
    prelude::*,
    text::{EditableText, FontCx, LayoutCx},
};

use super::Editor;

impl Editor {
    /// Applies the queued edits before UI layout.
    ///
    /// `EditableText` applies them after layout, so the colored copy of the
    /// text would lag one frame behind. Edits queued later in the frame are
    /// still applied by `EditableText`.
    pub(super) fn apply_edits(
        mut editors: Query<&mut EditableText, With<Editor>>,
        mut font_cx: ResMut<FontCx>,
        mut layout_cx: ResMut<LayoutCx>,
        mut clipboard: ResMut<Clipboard>,
    ) {
        for mut text in &mut editors {
            if text.pending_edits.is_empty() && text.pending_paste.is_none() {
                continue;
            }
            // The editor has no `EditableTextFilter`.
            text.apply_pending_edits(
                &mut font_cx.context,
                &mut layout_cx.0,
                &mut clipboard,
                |_| true,
            );
        }
    }
}
