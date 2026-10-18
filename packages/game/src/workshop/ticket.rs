use std::error::Error;

use bevy::prelude::*;
use ld60_devices::{ChipSpec, DeviceKind, RomKind};
use ld60_levels::{Level, LevelSource};

use super::job::Job;
use super::service;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::parts::paper::Paper;
use crate::parts::screen::UiCanvas;
use crate::player::focus::{Focus, FocusTarget, View};

const SHEET_SIZE: Vec2 = Vec2::new(0.16, 0.22);
const CANVAS_SIZE: UVec2 = UVec2::new(640, 880);
const BOARD_SIZE: Vec3 = Vec3::new(0.18, 0.005, 0.26);
/// The sheet sits a little towards the player, below the clip.
const SHEET_OFFSET: f32 = 0.012;
const SHEET_GAP: f32 = 0.0003;
const CLIP_SIZE: Vec3 = Vec3::new(0.07, 0.012, 0.025);
const BOARD: Color = Color::srgb_u8(0x7a, 0x5a, 0x3c);
const CLIP: Color = Color::srgb_u8(0xa8, 0xac, 0xb0);
const FOOTER: &str = "DSG SERVICE BENCH";
const LABEL_WIDTH: f32 = 120.0;
const VALUE_WIDTH: f32 = 440.0;

pub(crate) struct TicketPlugin;

impl Plugin for TicketPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(JobTicket::on_add).add_systems(
            Update,
            TicketStatus::update.run_if(resource_exists_and_changed::<Job>),
        );
    }
}

/// The job ticket for a device on the bench, on a clipboard with the
/// service sheet for its model below it.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct JobTicket {
    level: &'static LevelSource,
}

impl JobTicket {
    pub(crate) fn for_level(level: &'static LevelSource) -> Self {
        Self { level }
    }

    fn on_add(
        add: On<Add<JobTicket>>,
        tickets: Query<&JobTicket>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok(ticket) = tickets.get(add.entity) else {
            return;
        };
        let level = match ticket.level.load() {
            Ok(level) => level,
            Err(err) => {
                error!(
                    error = &err as &dyn Error,
                    "level {} does not load", ticket.level.path
                );
                return;
            }
        };

        let canvas = UiCanvas::spawn(&mut commands, &mut images, CANVAS_SIZE);
        let pages = vec![
            Paper::spawn_page(&mut commands, &canvas, (1, 2), FOOTER, |page| {
                Self::write(page, &level);
            }),
            Paper::spawn_page(
                &mut commands,
                &canvas,
                (2, 2),
                service::footer(level.device),
                |page| service::write(page, level.device),
            ),
        ];
        let board = Block::new(BOARD_SIZE);
        let clip = Block::new(CLIP_SIZE)
            .on_top_of(&board)
            .translated(Vec3::Z * (CLIP_SIZE.z - BOARD_SIZE.z) / 2.0);

        let sheet_center = Vec3::new(0.0, board.max().y + SHEET_GAP, SHEET_OFFSET);
        let mut sheet = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|clipboard| {
            clipboard.spawn(Shape::cuboid(&board, BOARD));
            sheet = clipboard
                .spawn(Paper::sheet(
                    &canvas,
                    SHEET_SIZE,
                    sheet_center,
                    &mut meshes,
                    &mut materials,
                ))
                .id();
            clipboard.spawn(Shape::cuboid(&clip, CLIP));
        });
        commands.entity(add.entity).insert((
            Paper::new(sheet, SHEET_SIZE, pages),
            FocusTarget {
                focus: Focus::Paper,
                view: View::Local(Paper::reading_view(sheet_center)),
            },
        ));
    }

    fn write(page: &mut ChildSpawnerCommands, level: &Level) {
        let device = match level.device {
            DeviceKind::Tally2 => "Tally 2 counter",
            DeviceKind::Summa6 => "Summa 6 adding machine",
        };
        let chip = match level.chip {
            ChipSpec::Calco(info) => {
                format!(
                    "CalCo rev {:?}, date code {}",
                    info.revision, info.date_code
                )
            }
        };
        let rom = match (level.rom.kind, level.rom.blank_available) {
            (RomKind::Eprom, true) => "EPROM, blank in stock",
            (RomKind::Eprom, false) => "EPROM, no blank. Burn over it.",
            (RomKind::Mask, _) => "Mask ROM, cannot be burned",
        };

        page.spawn(Paper::title("JOB TICKET"));
        page.spawn(Paper::faded("DSG service bench"));
        for (label, value) in [
            ("JOB", level.id.to_uppercase()),
            ("CUSTOMER", level.customer.name.clone()),
            ("DEVICE", device.into()),
            ("S/N", level.serial.clone()),
            ("CHIP", chip),
            ("ROM", rom.into()),
        ] {
            page.spawn(Paper::row([
                (label.into(), LABEL_WIDTH),
                (value, VALUE_WIDTH),
            ]));
        }
        page.spawn(Paper::heading("COMPLAINT"));
        page.spawn(Paper::text(format!("\"{}\"", level.customer.complaint)));
        page.spawn(Paper::heading("REMARKS"));
        for remark in &level.remarks {
            page.spawn(Paper::text(format!("- {remark}")));
        }
        page.spawn(Paper::heading("STATUS"));
        page.spawn((TicketStatus, Paper::text(TicketStatus::text(false))));
    }
}

/// The status line on the job ticket.
#[derive(Component, Debug)]
struct TicketStatus;

impl TicketStatus {
    fn text(repaired: bool) -> &'static str {
        if repaired {
            "Repaired. Ready for pickup."
        } else {
            "Open"
        }
    }

    fn update(job: Res<Job>, mut texts: Query<&mut Text, With<TicketStatus>>) {
        for mut text in &mut texts {
            text.0 = Self::text(job.repaired).into();
        }
    }
}
