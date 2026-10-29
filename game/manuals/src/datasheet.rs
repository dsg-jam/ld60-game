use bevy::prelude::*;
use ld60_calco::datasheet::{Group, INSTRUCTIONS, InstructionDoc, NOTES, PORTS};
use ld60_calco::{Erratum, Op, Revision};
use ld60_game_parts::paper::Paper;
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

    /// What the chip has to work with, in plain words.
    pub(super) fn registers(self, page: &mut ChildSpawnerCommands) {
        for (name, text, op) in [
            ("A", "datasheet-register-a", None),
            ("B", "datasheet-register-b", Some(Op::LdiB(0))),
            ("M0-MF", "datasheet-cells", None),
            ("CALL", "datasheet-register-stack", Some(Op::Call(0))),
        ] {
            if op.is_none_or(|op| self.revision.supports(op)) {
                page.spawn(Paper::row([
                    (Localized::verbatim(name), 90.0),
                    (Message::new(text).into(), 454.0),
                ]));
            }
        }
    }

    /// What the registers hold after a reset.
    pub(super) fn reset(self, page: &mut ChildSpawnerCommands) {
        let reset = if self.revision.supports(Op::LdiB(0)) {
            "datasheet-reset-b"
        } else {
            "datasheet-reset"
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
                (
                    Message::new(format!("datasheet-port-{}", port.name.to_lowercase())).into(),
                    404.0,
                ),
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
                    Self::row(doc).attribute("operation").into(),
                    Localized::verbatim(doc.z.symbol()),
                    Localized::verbatim(doc.c.symbol()),
                ]));
            }
        });
    }

    /// The instructions of `group` the revision has, under a heading, each
    /// with an example and what it does, and its notes under it.
    /// Whether the chip has any instruction of `group`.
    pub(super) fn has(self, group: Group) -> bool {
        self.docs(group).next().is_some()
    }

    pub(super) fn group(self, group: Group, page: &mut ChildSpawnerCommands) {
        let docs: Vec<_> = self.docs(group).collect();
        if docs.is_empty() {
            return;
        }
        page.spawn(Paper::heading(Message::new(Self::heading(group))));
        for doc in docs {
            page.spawn(Paper::row([
                (Localized::verbatim(doc.example), 130.0),
                (Self::row(doc).attribute("meaning").into(), 414.0),
            ]));
            if NOTES.contains(&doc.op) {
                page.spawn(Paper::faded(Self::row(doc).attribute("note")));
            }
        }
    }

    /// The words for the row of `doc` in the instruction table.
    fn row(doc: &InstructionDoc) -> Message {
        Message::new(format!(
            "datasheet-op-{}",
            doc.hex.to_lowercase().replace(' ', "-")
        ))
    }

    fn docs(self, group: Group) -> impl Iterator<Item = &'static InstructionDoc> {
        INSTRUCTIONS
            .iter()
            .filter(move |doc| doc.group == Some(group) && self.revision.supports(doc.op))
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
            let message = Message::new(format!("erratum-{}", erratum.id()));
            page.spawn(Paper::text(message.clone().attribute("title")));
            page.spawn(Paper::faded(
                Message::new("datasheet-affects")
                    .with("chips", message.clone().attribute("affects")),
            ));
            page.spawn(Paper::text(message.attribute("description")));
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::Numbers => "datasheet-group-numbers",
            Group::Keys => "datasheet-group-keys",
            Group::Jumps => "datasheet-group-jumps",
            Group::Math => "datasheet-group-math",
            Group::Routines => "datasheet-group-routines",
            Group::Loops => "datasheet-group-loops",
            Group::Lights => "datasheet-group-lights",
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::Locale;

    use super::*;

    #[test]
    fn every_table_formats_on_every_revision() {
        use Group::{Jumps, Keys, Lights, Loops, Math, Numbers, Routines};

        let locale = Locale::on_disk();
        for revision in Revision::ALL {
            let sheet = ChipSheet::new(revision);
            locale.assert_formats(Localized::spawned_by(|page| {
                sheet.registers(page);
                sheet.reset(page);
                sheet.ports(page);
                sheet.instructions(page);
                for group in [Numbers, Keys, Jumps, Math, Routines, Loops, Lights] {
                    sheet.group(group, page);
                }
                sheet.errata(page);
            }));
        }
    }

    #[test]
    fn every_note_is_under_an_instruction_row() {
        for op in NOTES {
            assert!(INSTRUCTIONS.iter().any(|doc| doc.op == *op), "{op:?}");
        }
    }
}
