use std::f32::consts::FRAC_PI_4;

use bevy::prelude::*;
use ld60_calco::datasheet::{INSTRUCTIONS, NOTES, PORTS};
use ld60_calco::{Addr, Cond, Erratum, Op, Revision};
use ld60_devices::ChipSpec;

use super::job::Job;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;
use crate::locale::{Localized, Message};
use crate::parts::paper::{PAPER, Paper};
use crate::parts::screen::UiCanvas;
use crate::player::focus::{Focus, FocusTarget, View};

const SHEET_SIZE: Vec2 = Vec2::new(0.16, 0.22);
const CANVAS_SIZE: UVec2 = UVec2::new(640, 880);
/// Pages under the top sheet, to make it look like a stack.
const STACK_SIZE: Vec3 = Vec3::new(0.16, 0.003, 0.22);
const STAPLE_SIZE: Vec3 = Vec3::new(0.012, 0.001, 0.002);
/// Keeps the top sheet from flickering against the stack.
const SHEET_GAP: f32 = 0.0002;
/// From the top left corner. The staple runs across the corner.
const STAPLE_INSET: f32 = 0.007;
/// The instruction table needs tighter rows to fit on one page.
const TABLE_ROW_GAP: f32 = 3.0;
const STAPLE: Color = Color::srgb_u8(0xa0, 0xa4, 0xa8);

pub(crate) struct DatasheetPlugin;

impl Plugin for DatasheetPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Datasheet::on_add).add_systems(
            Update,
            Datasheet::print_for_job
                .run_if(resource_exists_and_changed::<Job>)
                .before(Paper::show_current),
        );
    }
}

/// The stapled CalCo datasheet, lying face up. It shows the chip revision
/// of the current job.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Datasheet;

/// The canvas of a [`Datasheet`] and the revision its pages show.
#[derive(Component)]
struct Printed {
    canvas: UiCanvas,
    revision: Option<Revision>,
}

impl Datasheet {
    fn on_add(
        add: On<Add<Datasheet>>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let canvas = UiCanvas::spawn(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let stack = Block::new(STACK_SIZE);
        let staple = Block::new(STAPLE_SIZE)
            .on_top_of(&stack)
            .translated(Vec3::new(
                -SHEET_SIZE.x / 2.0 + STAPLE_INSET,
                SHEET_GAP,
                -SHEET_SIZE.y / 2.0 + STAPLE_INSET,
            ));

        let mut sheet = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|datasheet| {
            datasheet.spawn((Shape::cuboid(&stack, PAPER), Finish::Paper));
            sheet = datasheet
                .spawn(Paper::sheet(
                    &canvas,
                    SHEET_SIZE,
                    Vec3::Y * (stack.max().y + SHEET_GAP),
                    &mut meshes,
                    &mut materials,
                ))
                .id();
            datasheet
                .spawn((Shape::cuboid(&staple, STAPLE), Finish::Metal))
                .insert(
                    staple
                        .transform()
                        .with_rotation(Quat::from_rotation_y(FRAC_PI_4)),
                );
        });
        commands.entity(add.entity).insert((
            Paper::new(sheet, SHEET_SIZE, Vec::new()),
            Printed {
                canvas,
                revision: None,
            },
            FocusTarget {
                prompt: "click-datasheet",
                focus: Focus::Paper,
                view: View::Local(Paper::reading_view(Vec3::ZERO)),
            },
        ));
    }

    /// Reprints the pages when a job brings another chip revision.
    fn print_for_job(
        job: Res<Job>,
        mut sheets: Query<(&mut Paper, &mut Printed)>,
        mut commands: Commands,
    ) {
        let ChipSpec::Calco(chip) = job.level.chip;
        for (mut paper, mut printed) in &mut sheets {
            if printed.revision == Some(chip.revision) {
                continue;
            }
            printed.revision = Some(chip.revision);
            let pages = Self::print(&mut commands, &printed.canvas, chip.revision);
            for page in paper.replace_pages(pages) {
                commands.entity(page).despawn();
            }
        }
    }

    /// The contents of each page.
    const PAGES: [fn(&mut ChildSpawnerCommands, Revision); 4] = [
        Self::overview,
        Self::instructions,
        Self::conditions,
        Self::errata,
    ];

    fn footer(revision: Revision) -> Message {
        Message::new("datasheet-footer").with("revision", revision.to_string())
    }

    fn print(commands: &mut Commands, canvas: &UiCanvas, revision: Revision) -> Vec<Entity> {
        let footer = Self::footer(revision);
        Self::PAGES
            .iter()
            .enumerate()
            .map(|(index, content)| {
                Paper::spawn_page(
                    commands,
                    canvas,
                    (index + 1, Self::PAGES.len()),
                    footer.clone(),
                    |page| content(page, revision),
                )
            })
            .collect()
    }

    fn overview(page: &mut ChildSpawnerCommands, revision: Revision) {
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
                    (Message::new(text).into(), 480.0),
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
                (Localized::verbatim(port.function), 420.0),
            ]));
        }
    }

    fn instructions(page: &mut ChildSpawnerCommands, revision: Revision) {
        const WIDTHS: [f32; 5] = [70.0, 120.0, 314.0, 28.0, 28.0];
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

    fn conditions(page: &mut ChildSpawnerCommands, revision: Revision) {
        page.spawn(Paper::heading(Message::new("datasheet-conditions")));
        for cond in Cond::ALL
            .into_iter()
            .filter(|&cond| revision.supports(Op::Jump(cond, 0)))
        {
            page.spawn(Paper::row([
                (Localized::verbatim(format!("{:X}", cond.code())), 40.0),
                (Localized::verbatim(cond.mnemonic()), 80.0),
                (Self::condition(cond).into(), 440.0),
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

    fn errata(page: &mut ChildSpawnerCommands, revision: Revision) {
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
            locale.assert_formats([Datasheet::footer(revision)]);
            for content in Datasheet::PAGES {
                locale.assert_formats(Localized::spawned_by(|page| content(page, revision)));
            }
        }
    }
}
