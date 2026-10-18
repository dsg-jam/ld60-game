use bevy::prelude::*;
use ld60_devices::{ChipSpec, DeviceKind, RomKind};
use ld60_levels::Level;

use super::job::Job;
use super::service;
use crate::audio::{Play, Sound};
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;
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
/// Top left corner of the stamp on the canvas.
const STAMP_POSITION: Vec2 = Vec2::new(300.0, 520.0);
/// In degrees, clockwise.
const STAMP_TILT: f32 = -12.0;
/// Seconds until the camera reaches the ticket.
const STAMP_DELAY: f32 = 0.55;
const STAMP_DROP_TIME: f32 = 0.12;
/// How big the stamp looks while it is still above the paper.
const STAMP_START_SCALE: f32 = 1.7;
const STAMP_INK: Color = Color::srgba(0.72, 0.12, 0.1, 0.8);
const LABEL_WIDTH: f32 = 120.0;
const VALUE_WIDTH: f32 = 440.0;

pub(crate) struct TicketPlugin;

impl Plugin for TicketPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(JobTicket::on_add).add_systems(
            Update,
            (
                TicketStatus::update.run_if(resource_exists_and_changed::<Job>),
                Landing::animate,
            ),
        );
    }
}

/// The job ticket for a device on the bench, on a clipboard with the
/// service sheet for its model below it.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct JobTicket {
    level: Level,
}

impl JobTicket {
    pub(crate) fn for_level(level: &Level) -> Self {
        Self {
            level: level.clone(),
        }
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
        let level = &ticket.level;
        let canvas = UiCanvas::spawn(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let pages = vec![
            Paper::spawn_page(&mut commands, &canvas, (1, 3), FOOTER, |page| {
                Self::write(page, level);
            }),
            Paper::spawn_page(
                &mut commands,
                &canvas,
                (2, 3),
                service::footer(level.device),
                |page| service::write(page, level.board()),
            ),
            Paper::spawn_page(
                &mut commands,
                &canvas,
                (3, 3),
                service::footer(level.device),
                |page| service::write_line_tests(page, level.device),
            ),
        ];
        let board = Block::new(BOARD_SIZE);
        let clip = Block::new(CLIP_SIZE)
            .on_top_of(&board)
            .translated(Vec3::Z * (CLIP_SIZE.z - BOARD_SIZE.z) / 2.0);

        let sheet_center = Vec3::new(0.0, board.max().y + SHEET_GAP, SHEET_OFFSET);
        let mut sheet = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|clipboard| {
            clipboard.spawn((Shape::cuboid(&board, BOARD), Finish::Wood));
            sheet = clipboard
                .spawn(Paper::sheet(
                    &canvas,
                    SHEET_SIZE,
                    sheet_center,
                    &mut meshes,
                    &mut materials,
                ))
                .id();
            clipboard.spawn((Shape::cuboid(&clip, CLIP), Finish::Metal));
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
            ("BOARD", level.board().name()),
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
        page.spawn((TicketStatus, Paper::text(TicketStatus::text(None))));
        page.spawn(ShopStamp::bundle());
    }
}

/// The status line on the job ticket.
#[derive(Component, Debug)]
struct TicketStatus;

impl TicketStatus {
    fn text(job: Option<&Job>) -> &'static str {
        match job {
            Some(job) if job.stamped => "Done. Ring the bell for pickup.",
            Some(job) if job.repaired => "Repaired. Stamp the ticket.",
            _ => "Open",
        }
    }

    fn update(
        job: Res<Job>,
        mut texts: Query<&mut Text, With<TicketStatus>>,
        mut stamps: Query<(Entity, &mut Visibility, Has<Landing>), With<ShopStamp>>,
        mut commands: Commands,
    ) {
        for mut text in &mut texts {
            text.0 = Self::text(Some(&job)).into();
        }
        for (stamp, mut visibility, landing) in &mut stamps {
            if !job.stamped {
                *visibility = Visibility::Hidden;
            } else if *visibility == Visibility::Hidden && !landing {
                commands.entity(stamp).insert(Landing::default());
            }
        }
    }
}

/// The shop stamp coming down onto the ticket.
///
/// It waits for the camera to reach the ticket, then drops from above and
/// thunks when it hits the paper.
#[derive(Component, Debug, Default)]
struct Landing {
    elapsed: f32,
}

impl Landing {
    fn animate(
        time: Res<Time>,
        mut stamps: Query<(Entity, &mut Landing, &mut Visibility, &mut UiTransform)>,
        mut commands: Commands,
    ) {
        for (stamp, mut landing, mut visibility, mut transform) in &mut stamps {
            landing.elapsed += time.delta_secs();
            let t = (landing.elapsed - STAMP_DELAY) / STAMP_DROP_TIME;
            if t < 0.0 {
                continue;
            }
            *visibility = Visibility::Inherited;
            // Speeds up as it falls, like a hand pressing down.
            let fall = t.min(1.0).powi(2);
            transform.scale = Vec2::splat(STAMP_START_SCALE + (1.0 - STAMP_START_SCALE) * fall);
            if t >= 1.0 {
                commands.entity(stamp).remove::<Landing>();
                commands.trigger(Play(Sound::Thunk));
            }
        }
    }
}

/// The shop's rubber stamp, printed on the ticket of a finished job.
#[derive(Component, Debug)]
struct ShopStamp;

impl ShopStamp {
    fn bundle() -> impl Bundle {
        let ink = |size: f32, text: &str| {
            (
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(size),
                    ..default()
                },
                TextColor(STAMP_INK),
            )
        };
        (
            ShopStamp,
            Node {
                position_type: PositionType::Absolute,
                left: px(STAMP_POSITION.x),
                top: px(STAMP_POSITION.y),
                padding: px(5).all(),
                border: px(5).all(),
                border_radius: BorderRadius::all(px(22)),
                ..default()
            },
            BorderColor::all(STAMP_INK),
            UiTransform::from_rotation(Rot2::degrees(STAMP_TILT)),
            Visibility::Hidden,
            children![(
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(4),
                    padding: UiRect::axes(px(22), px(10)),
                    border: px(2).all(),
                    border_radius: BorderRadius::all(px(16)),
                    ..default()
                },
                BorderColor::all(STAMP_INK),
                children![
                    ink(20.0, "AUTHORIZED"),
                    ink(64.0, "DSG"),
                    ink(22.0, "REPAIR SHOP"),
                    (
                        Node {
                            width: percent(100),
                            height: px(2),
                            ..default()
                        },
                        BackgroundColor(STAMP_INK),
                    ),
                    ink(16.0, "FIXED WITH CARE"),
                ],
            )],
        )
    }
}
