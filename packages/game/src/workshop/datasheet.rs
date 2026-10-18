use std::f32::consts::FRAC_PI_4;

use bevy::prelude::*;
use ld60_calco::datasheet::{INSTRUCTIONS, NOTES, PORTS};
use ld60_calco::{Cond, Erratum, Op, Revision};
use ld60_devices::ChipSpec;

use super::job::Job;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;
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
            Datasheet::print_for_job.run_if(resource_exists_and_changed::<Job>),
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

    fn print(commands: &mut Commands, canvas: &UiCanvas, revision: Revision) -> Vec<Entity> {
        let footer = format!("DSG ELECTRONICS   CALCO REV {revision}");
        let contents: [fn(&mut ChildSpawnerCommands, Revision); _] = [
            Self::overview,
            Self::instructions,
            Self::conditions,
            Self::errata,
        ];
        contents
            .iter()
            .enumerate()
            .map(|(index, content)| {
                Paper::spawn_page(
                    commands,
                    canvas,
                    (index + 1, contents.len()),
                    &footer,
                    |page| content(page, revision),
                )
            })
            .collect()
    }

    fn overview(page: &mut ChildSpawnerCommands, revision: Revision) {
        page.spawn(Paper::title("CALCO"));
        page.spawn(Paper::faded(
            "8-bit calculator coprocessor. One decimal digit per byte.",
        ));
        page.spawn(Paper::heading("REGISTERS"));
        for (name, text, op) in [
            ("A", "Accumulator. All ALU results go here.", None),
            (
                "B",
                "Pointer for [B+n], counter for DJNZ.",
                Some(Op::LdiB(0)),
            ),
            ("PC", "Program counter. Wraps from FF to 00.", None),
            ("Z, C", "Zero and carry flags. C is set on borrow.", None),
            (
                "Stack",
                "Two return addresses for CALL and RET.",
                Some(Op::Call(0)),
            ),
        ] {
            if op.is_none_or(|op| revision.supports(op)) {
                page.spawn(Paper::row([(name.into(), 80.0), (text.into(), 480.0)]));
            }
        }
        page.spawn(Paper::heading("MEMORY"));
        page.spawn(Paper::text(
            "ROM: 256 bytes, 00 to FF. Code cannot read it as data.",
        ));
        page.spawn(Paper::text("RAM: 16 cells, M0 to MF."));
        page.spawn(Paper::text(
            "Reset: PC, A, B, Z and C are 0. RAM and stack are undefined.",
        ));
        page.spawn(Paper::heading("PORTS"));
        for port in PORTS.iter().filter(|port| revision.supports(port.op())) {
            let direction = if port.output { "OUT" } else { "IN" };
            page.spawn(Paper::row([
                (port.name.into(), 60.0),
                (direction.into(), 50.0),
                (format!("{:X}", port.port), 30.0),
                (port.function.into(), 420.0),
            ]));
        }
    }

    fn instructions(page: &mut ChildSpawnerCommands, revision: Revision) {
        const WIDTHS: [f32; 5] = [70.0, 120.0, 314.0, 28.0, 28.0];
        let row = |cells: [String; 5]| {
            let mut widths = WIDTHS.into_iter();
            Paper::row(cells.map(|cell| (cell, widths.next().unwrap_or_default())))
        };
        page.spawn(Paper::heading("INSTRUCTION SET"));
        page.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(TABLE_ROW_GAP),
            ..default()
        })
        .with_children(|table| {
            table.spawn(row(
                ["HEX", "SYNTAX", "OPERATION", "Z", "C"].map(String::from)
            ));
            for doc in INSTRUCTIONS.iter().filter(|doc| revision.supports(doc.op)) {
                table.spawn(row([
                    doc.hex.into(),
                    doc.syntax.into(),
                    doc.operation.into(),
                    doc.z.symbol().into(),
                    doc.c.symbol().into(),
                ]));
            }
        });
        page.spawn(Paper::faded(
            "* from result   - unchanged   0 cleared   1 set",
        ));
        page.spawn(Paper::faded(
            "n: cell or offset   ii: byte   aa: address   p: port",
        ));
    }

    fn conditions(page: &mut ChildSpawnerCommands, revision: Revision) {
        page.spawn(Paper::heading("CONDITIONS"));
        for cond in Cond::ALL
            .into_iter()
            .filter(|&cond| revision.supports(Op::Jump(cond, 0)))
        {
            page.spawn(Paper::row([
                (format!("{:X}", cond.code()), 40.0),
                (cond.mnemonic().into(), 80.0),
                (format!("jumps if {}", cond.description()), 440.0),
            ]));
        }
        page.spawn(Paper::faded(
            "After CMP or CPI, JC jumps if A is lower and JZ if A is equal.",
        ));
        let notes: Vec<_> = NOTES
            .iter()
            .filter(|note| revision.supports(note.op))
            .collect();
        if !notes.is_empty() {
            page.spawn(Paper::heading("NOTES"));
        }
        for note in notes {
            page.spawn(Paper::text(format!("- {}", note.text)));
        }
    }

    fn errata(page: &mut ChildSpawnerCommands, revision: Revision) {
        page.spawn(Paper::heading("ERRATA"));
        let errata: Vec<_> = Erratum::ALL
            .iter()
            .filter(|erratum| erratum.revision() == revision)
            .collect();
        if errata.is_empty() {
            page.spawn(Paper::text(format!("No known bugs in Rev {revision}.")));
        }
        for erratum in errata {
            page.spawn(Paper::text(erratum.title().to_uppercase()));
            page.spawn(Paper::faded(format!(
                "Affects: {}",
                erratum.affected_chips()
            )));
            page.spawn(Paper::text(erratum.description()));
        }
    }
}
