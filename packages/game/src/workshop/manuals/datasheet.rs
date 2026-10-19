use bevy::prelude::*;
use ld60_calco::datasheet::{INSTRUCTIONS, NOTES, PORTS};
use ld60_calco::{Addr, Cond, Erratum, Op, Revision};

use crate::locale::{Localized, Message};
use crate::parts::paper::Paper;

/// The instruction table needs tighter rows to fit on one page.
const TABLE_ROW_GAP: f32 = 3.0;

/// The CalCo datasheet of one chip revision, and its errata pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ChipSheet {
    revision: Revision,
}

impl ChipSheet {
    pub(super) fn new(revision: Revision) -> Self {
        Self { revision }
    }

    /// The contents of each page of the datasheet.
    pub(super) const PAGES: [fn(&mut ChildSpawnerCommands, Self); 3] =
        [Self::overview, Self::instructions, Self::conditions];

    pub(super) fn footer(self) -> Message {
        Message::new("datasheet-footer").with("revision", self.revision.to_string())
    }

    fn overview(page: &mut ChildSpawnerCommands, sheet: Self) {
        let revision = sheet.revision;
        page.spawn(Paper::title(Localized::verbatim("CALCO")));
        page.spawn(Paper::faded(Message::new("datasheet-subtitle")));
        page.spawn(Paper::heading(Message::new("datasheet-registers")));
        for (name, text, op) in [
            (Localized::verbatim("A"), "datasheet-register-a", None),
            (
                Localized::verbatim("B"),
                "datasheet-register-b",
                Some(Op::LdiB(0)),
            ),
            (Localized::verbatim("PC"), "datasheet-register-pc", None),
            (
                Localized::verbatim("Z, C"),
                "datasheet-register-flags",
                None,
            ),
            (
                Message::new("datasheet-stack").into(),
                "datasheet-register-stack",
                Some(Op::Call(0)),
            ),
        ] {
            if op.is_none_or(|op| revision.supports(op)) {
                page.spawn(Paper::row([
                    (name, 80.0),
                    (Message::new(text).into(), 464.0),
                ]));
            }
        }
        page.spawn(Paper::heading(Message::new("datasheet-memory")));
        page.spawn(Paper::text(Message::new("datasheet-rom")));
        page.spawn(Paper::text(Message::new("datasheet-ram")));
        let reset = match (
            revision.supports(Op::LdiB(0)),
            revision.supports(Op::Call(0)),
        ) {
            (true, true) => "datasheet-reset-b-stack",
            (true, false) => "datasheet-reset-b",
            (false, true) => "datasheet-reset-stack",
            (false, false) => "datasheet-reset",
        };
        page.spawn(Paper::text(Message::new(reset)));
        page.spawn(Paper::heading(Message::new("datasheet-ports")));
        for port in PORTS.iter().filter(|port| revision.supports(port.op())) {
            let direction = if port.output { "OUT" } else { "IN" };
            page.spawn(Paper::row([
                (Localized::verbatim(port.name), 60.0),
                (Localized::verbatim(direction), 50.0),
                (Localized::verbatim(format!("{:X}", port.port)), 30.0),
                (Localized::verbatim(port.function), 404.0),
            ]));
        }
    }

    fn instructions(page: &mut ChildSpawnerCommands, sheet: Self) {
        let revision = sheet.revision;
        const WIDTHS: [f32; 5] = [70.0, 120.0, 298.0, 28.0, 28.0];
        let row = |cells: [Localized; 5]| {
            let mut widths = WIDTHS.into_iter();
            Paper::row(cells.map(|cell| (cell, widths.next().unwrap_or_default())))
        };
        page.spawn(Paper::heading(Message::new("datasheet-instructions")));
        page.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(TABLE_ROW_GAP),
            ..default()
        })
        .with_children(|table| {
            table.spawn(row([
                Localized::verbatim("HEX"),
                Message::new("datasheet-syntax").into(),
                Message::new("datasheet-operation").into(),
                Localized::verbatim("Z"),
                Localized::verbatim("C"),
            ]));
            for doc in INSTRUCTIONS.iter().filter(|doc| revision.supports(doc.op)) {
                table.spawn(row([
                    Localized::verbatim(doc.hex),
                    Localized::verbatim(doc.syntax),
                    Localized::verbatim(doc.operation),
                    Localized::verbatim(doc.z.symbol()),
                    Localized::verbatim(doc.c.symbol()),
                ]));
            }
        });
        page.spawn(Paper::faded(Message::new("datasheet-flag-legend")));
        page.spawn(Paper::faded(Message::new("datasheet-operand-legend")));
    }

    fn conditions(page: &mut ChildSpawnerCommands, sheet: Self) {
        let revision = sheet.revision;
        page.spawn(Paper::heading(Message::new("datasheet-conditions")));
        for cond in Cond::ALL
            .into_iter()
            .filter(|&cond| revision.supports(Op::Jump(cond, 0)))
        {
            page.spawn(Paper::row([
                (Localized::verbatim(format!("{:X}", cond.code())), 40.0),
                (Localized::verbatim(cond.mnemonic()), 80.0),
                (Self::condition(cond).into(), 424.0),
            ]));
        }
        let compares = if revision.supports(Op::Cmp(Addr::Direct(0))) {
            "datasheet-compare-cmp"
        } else {
            "datasheet-compare-cpi"
        };
        page.spawn(Paper::faded(Message::new(compares)));
        let notes: Vec<_> = NOTES
            .iter()
            .filter(|note| revision.supports(note.op))
            .collect();
        if !notes.is_empty() {
            page.spawn(Paper::heading(Message::new("datasheet-notes")));
        }
        for note in notes {
            page.spawn(Paper::text(
                Message::new("paper-bullet").with("text", note.text),
            ));
        }
    }

    /// When the jump of `cond` is taken.
    fn condition(cond: Cond) -> Message {
        Message::new(match cond {
            Cond::Always => "datasheet-condition-always",
            Cond::Zero => "datasheet-condition-zero",
            Cond::NotZero => "datasheet-condition-not-zero",
            Cond::Carry => "datasheet-condition-carry",
            Cond::NoCarry => "datasheet-condition-no-carry",
        })
    }

    /// The errata page of the revision.
    pub(super) fn errata(page: &mut ChildSpawnerCommands, sheet: Self) {
        let revision = sheet.revision;
        page.spawn(Paper::heading(Message::new("datasheet-errata")));
        let errata: Vec<_> = Erratum::ALL
            .iter()
            .filter(|erratum| erratum.revision() == revision)
            .collect();
        if errata.is_empty() {
            page.spawn(Paper::text(
                Message::new("datasheet-no-errata").with("revision", revision.to_string()),
            ));
        }
        for erratum in errata {
            page.spawn(Paper::text(Localized::verbatim(
                erratum.title().to_uppercase(),
            )));
            page.spawn(Paper::faded(
                Message::new("datasheet-affects").with("chips", erratum.affected_chips()),
            ));
            page.spawn(Paper::text(Localized::verbatim(erratum.description())));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::Locale;

    #[test]
    fn every_page_formats() {
        let locale = Locale::on_disk();
        for revision in Revision::ALL {
            let sheet = ChipSheet::new(revision);
            locale.assert_formats([sheet.footer()]);
            for content in ChipSheet::PAGES {
                locale.assert_formats(Localized::spawned_by(|page| content(page, sheet)));
            }
            locale.assert_formats(Localized::spawned_by(|page| {
                ChipSheet::errata(page, sheet);
            }));
        }
    }
}
