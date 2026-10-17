use std::time::Duration;

use bevy::{
    picking::{PickingSettings, events::PointerPress, pointer::PointerButton},
    prelude::*,
    text::{EditableText, TextEdit},
    ui::{ComputedUiRenderTargetInfo, UiGlobalTransform},
};

use super::Editor;

/// Presses farther apart than this, in canvas pixels, start a new click.
const SAME_SPOT: f32 = 4.0;

/// Click counting for the editor.
///
/// Bevy counts presses on the same entity within the multi-click interval,
/// no matter where they land, and `TextInput` selects all text on a third
/// click. We also require the same spot, stop at a double click, and
/// rewrite the edit `TextInput` queued when our count differs from Bevy's.
#[derive(Default)]
pub(super) struct Clicks {
    last: Option<(Vec2, Duration)>,
}

impl Clicks {
    pub(super) fn correct_edits(
        mut clicks: Local<Clicks>,
        mut presses: MessageReader<PointerPress>,
        time: Res<Time<Real>>,
        settings: Res<PickingSettings>,
        ui_scale: Res<UiScale>,
        mut editors: Query<
            (
                &mut EditableText,
                &ComputedNode,
                &UiGlobalTransform,
                &ComputedUiRenderTargetInfo,
            ),
            With<Editor>,
        >,
    ) {
        for press in presses.read() {
            let Ok((mut text, node, transform, target)) = editors.get_mut(press.entity) else {
                continue;
            };
            if press.button != PointerButton::Primary {
                continue;
            }
            let position = press.pointer.position;
            let count = clicks.register(position, time.elapsed(), settings.multi_click_interval);
            if count == press.count {
                continue;
            }

            // Same math as `TextInput`, to get the press in text coordinates.
            let content = node.content_box();
            let Some(local) = transform.try_inverse().map(|inverse| {
                inverse.transform_point2(position * target.scale_factor() / ui_scale.0)
            }) else {
                continue;
            };
            if !content.contains(local) {
                continue;
            }
            let point = local - content.min + text.viewport.offset;

            let edit = if count == 1 {
                TextEdit::MoveToPoint(point)
            } else {
                TextEdit::SelectWordAtPoint(point)
            };
            let queued = text.pending_edits.iter().rposition(|edit| {
                matches!(
                    edit,
                    TextEdit::MoveToPoint(_)
                        | TextEdit::ShiftClickExtension(_)
                        | TextEdit::SelectWordAtPoint(_)
                        | TextEdit::SelectAll
                )
            });
            if let Some(index) = queued {
                text.pending_edits[index] = edit;
            }
        }
    }

    /// Records a press and returns the click count, 1 or 2.
    fn register(&mut self, position: Vec2, now: Duration, interval: Duration) -> u8 {
        let repeat = self
            .last
            .is_some_and(|(last, at)| now - at <= interval && last.distance(position) <= SAME_SPOT);
        self.last = Some((position, now));
        if repeat { 2 } else { 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_only_presses_on_the_same_spot() {
        let mut clicks = Clicks::default();
        let ms = Duration::from_millis;
        let interval = ms(500);
        assert_eq!(clicks.register(Vec2::ZERO, ms(0), interval), 1);
        assert_eq!(clicks.register(Vec2::ONE, ms(100), interval), 2);
        assert_eq!(clicks.register(Vec2::ONE, ms(200), interval), 2);
        assert_eq!(clicks.register(Vec2::new(50.0, 0.0), ms(300), interval), 1);
        assert_eq!(clicks.register(Vec2::new(50.0, 0.0), ms(900), interval), 1);
    }
}
