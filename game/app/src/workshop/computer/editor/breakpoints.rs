use std::collections::BTreeSet;

use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
    text::EditableText,
    ui::{ComputedUiRenderTargetInfo, UiGlobalTransform},
};
use ld60_locale::{Localized, Message};
use ld60_machine::Assembly;

use super::Editor;
use super::footer::{Command, Run, Status};
use super::highlight::Highlight;
use super::listing::Listing;
use crate::workshop::job::{Job, JobId};
use crate::workshop::run::{ComputerTool, Run as ShopRun};
use ld60_game_device::{Connected, Emulator};

/// The 0-based source lines where the chip stops for the debugger.
///
/// The player sets them by clicking the margin left of a line or with the
/// footer command on the caret line.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub(super) struct Breakpoints(BTreeSet<usize>);

impl Breakpoints {
    pub(super) fn lines(&self) -> impl Iterator<Item = usize> {
        self.0.iter().copied()
    }

    /// The ROM addresses of the breakpoints in `assembly`.
    pub(super) fn addresses(&self, assembly: &Assembly) -> Vec<u16> {
        self.lines()
            .filter_map(|line| Self::address(assembly, line))
            .collect()
    }

    /// The address of the first instruction on 0-based `line`.
    pub(super) fn address(assembly: &Assembly, line: usize) -> Option<u16> {
        assembly
            .lines
            .iter()
            .find(|source| source.line == line + 1 && source.len > 0)
            .map(|source| source.addr)
    }

    /// Sets or clears the breakpoint on `line`. Only lines with code in the
    /// chip take one.
    pub(super) fn toggle(&mut self, line: usize, listing: &Listing) -> Message {
        let shown = line + 1;
        if self.0.remove(&line) {
            return Message::new("editor-break-cleared").with("line", shown);
        }
        let has_code = listing
            .assembly()
            .is_some_and(|assembly| Self::address(assembly, line).is_some());
        if !has_code {
            return Message::new("editor-break-no-code").with("line", shown);
        }
        self.0.insert(line);
        Message::new("editor-break-set").with("line", shown)
    }

    /// Line numbers belong to the firmware of one job.
    pub(super) fn follow_job(
        job: Option<Res<Job>>,
        mut loaded: Local<Option<JobId>>,
        mut breakpoints: ResMut<Breakpoints>,
    ) {
        let id = job.as_ref().map(|job| job.id);
        if id != *loaded {
            *loaded = id;
            breakpoints.0.clear();
        }
    }

    /// Gives the chip the addresses of the breakpoints.
    pub(super) fn arm(
        breakpoints: Res<Breakpoints>,
        listing: Res<Listing>,
        mut emulators: Query<&mut Emulator, With<Connected>>,
    ) {
        let addresses = listing
            .assembly()
            .map(|assembly| breakpoints.addresses(assembly))
            .unwrap_or_default();
        for mut emulator in &mut emulators {
            if emulator.breakpoints() != addresses {
                emulator.set_breakpoints(addresses.clone());
            }
        }
    }

    pub(super) fn toggle_on_run(
        run: On<Run>,
        editor: Single<&EditableText, With<Editor>>,
        listing: Res<Listing>,
        mut breakpoints: ResMut<Breakpoints>,
        mut status: Single<&mut Localized, With<Status>>,
    ) {
        if run.0 != Command::Break {
            return;
        }
        let editor = editor.editor();
        let source = editor.raw_text();
        let caret = editor.raw_selection().focus().index();
        let line = source.get(..caret).unwrap_or(source).matches('\n').count();
        **status = breakpoints.toggle(line, &listing).into();
    }

    /// A click in the margin left of the editor text toggles the breakpoint
    /// on that line.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    pub(super) fn toggle_on_margin(
        mut presses: MessageReader<PointerPress>,
        shop: Res<ShopRun>,
        ui_scale: Res<UiScale>,
        editor: Single<
            (
                Entity,
                &EditableText,
                &ComputedNode,
                &UiGlobalTransform,
                &ComputedUiRenderTargetInfo,
            ),
            With<Editor>,
        >,
        highlight: Single<&Highlight>,
        listing: Res<Listing>,
        mut breakpoints: ResMut<Breakpoints>,
        mut status: Single<&mut Localized, With<Status>>,
    ) {
        let (entity, text, node, transform, target) = *editor;
        for press in presses.read() {
            if press.entity != entity
                || press.button != PointerButton::Primary
                || !shop.has(ComputerTool::Breakpoints)
            {
                continue;
            }
            // Same math as `Clicks`, to get the press in node coordinates.
            let position = press.pointer.position * target.scale_factor() / ui_scale.0;
            let Some(local) = transform
                .try_inverse()
                .map(|inverse| inverse.transform_point2(position))
            else {
                continue;
            };
            let content = node.content_box();
            if local.x >= content.min.x || local.y < content.min.y || local.y > content.max.y {
                continue;
            }
            let Some(line) = highlight.line_at(local.y - content.min.y + text.viewport.offset.y)
            else {
                continue;
            };
            **status = breakpoints.toggle(line, &listing).into();
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::Locale;
    use ld60_machine::Isa;

    use super::*;

    fn listing() -> Listing {
        let isa = ld60_calco::Revision::C.isa();
        Listing::new(isa.assemble("start:\n    NOP\n\n    JMP start\n").unwrap())
    }

    #[test]
    fn takes_only_lines_with_code() {
        let listing = listing();
        let mut breakpoints = Breakpoints::default();
        let messages = [
            breakpoints.toggle(1, &listing),
            breakpoints.toggle(2, &listing),
            breakpoints.toggle(3, &listing),
        ];
        assert_eq!(breakpoints.lines().collect::<Vec<_>>(), [1, 3]);
        assert_eq!(breakpoints.addresses(listing.assembly().unwrap()), [0, 1]);
        let cleared = breakpoints.toggle(1, &listing);
        assert_eq!(breakpoints.lines().collect::<Vec<_>>(), [3]);
        Locale::on_disk().assert_formats(messages.into_iter().chain([cleared]));
    }
}
