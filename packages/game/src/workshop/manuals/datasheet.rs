use bevy::prelude::*;
use ld60_calco::datasheet::{INSTRUCTIONS, NOTES, PORTS};
use ld60_calco::{Addr, Cond, Erratum, Op, Revision};
use ld60_core::parts::paper::Paper;
use ld60_locale::{Localized, Message};

/// The tables of the CalCo datasheet and its errata, with what one chip
/// revision has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ChipSheet {
    revision: Revision,
}

impl ChipSheet {
    pub(super) fn new(revision: Revision) -> Self {
        Self { revision }
    }

    pub(super) fn registers(self, page: &mut ChildSpawnerCommands) {
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
            if op.is_none_or(|op| self.revision.supports(op)) {
                page.spawn(Paper::row([
                    (name, 80.0),
                    (Message::new(text).into(), 464.0),
                ]));
            }
        }
    }

    /// What the registers hold after a reset.
    pub(super) fn reset(self, page: &mut ChildSpawnerCommands) {
        let reset = match (
            self.revision.supports(Op::LdiB(0)),
            self.revision.supports(Op::Call(0)),
        ) {
            (true, true) => "datasheet-reset-b-stack",
            (true, false) => "datasheet-reset-b",
            (false, true) => "datasheet-reset-stack",
            (false, false) => "datasheet-reset",
        };
        page.spawn(Paper::text(Message::new(reset)));
    }

    pub(super) fn ports(self, page: &mut ChildSpawnerCommands) {
        for port in PORTS
            .iter()
            .filter(|port| self.revision.supports(port.op()))
        {
            let direction = if port.output { "OUT" } else { "IN" };
            page.spawn(Paper::row([
                (Localized::verbatim(port.name), 60.0),
                (Localized::verbatim(direction), 50.0),
                (Localized::verbatim(format!("{:X}", port.port)), 30.0),
                (Localized::verbatim(port.function), 404.0),
            ]));
        }
    }

    pub(super) fn instructions(self, page: &mut ChildSpawnerCommands) {
        const WIDTHS: [f32; 5] = [70.0, 120.0, 298.0, 28.0, 28.0];
        let row = |cells: [Localized; 5]| {
            let mut widths = WIDTHS.into_iter();
            Paper::row(cells.map(|cell| (cell, widths.next().unwrap_or_default())))
        };
        // No gaps between the rows, so the table fits on one page.
        page.spawn(Node {
            flex_direction: FlexDirection::Column,
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
            for doc in INSTRUCTIONS
                .iter()
                .filter(|doc| self.revision.supports(doc.op))
            {
                table.spawn(row([
                    Localized::verbatim(doc.hex),
                    Localized::verbatim(doc.syntax),
                    Localized::verbatim(doc.operation),
                    Localized::verbatim(doc.z.symbol()),
                    Localized::verbatim(doc.c.symbol()),
                ]));
            }
        });
    }

    /// The conditions the jumps of the revision can test.
    pub(super) fn conditions(self, page: &mut ChildSpawnerCommands) {
        for cond in Cond::ALL
            .into_iter()
            .filter(|&cond| self.revision.supports(Op::Jump(cond, 0)))
        {
            page.spawn(Paper::row([
                (Localized::verbatim(format!("{:X}", cond.code())), 40.0),
                (Localized::verbatim(cond.mnemonic()), 80.0),
                (Self::condition(cond).into(), 424.0),
            ]));
        }
    }

    /// How to compare before a jump.
    pub(super) fn compare(self, page: &mut ChildSpawnerCommands) {
        let compares = if self.revision.supports(Op::Cmp(Addr::Direct(0))) {
            "datasheet-compare-cmp"
        } else {
            "datasheet-compare-cpi"
        };
        page.spawn(Paper::faded(Message::new(compares)));
    }

    /// The notes on what the revision has, under a heading, if there are any.
    pub(super) fn notes(self, page: &mut ChildSpawnerCommands) {
        let notes: Vec<_> = NOTES
            .iter()
            .filter(|note| self.revision.supports(note.op))
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

    /// The known bugs of the revision.
    pub(super) fn errata(self, page: &mut ChildSpawnerCommands) {
        let errata: Vec<_> = Erratum::ALL
            .iter()
            .filter(|erratum| erratum.revision() == self.revision)
            .collect();
        if errata.is_empty() {
            page.spawn(Paper::text(
                Message::new("datasheet-no-errata").with("revision", self.revision.to_string()),
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
}
