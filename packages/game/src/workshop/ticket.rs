use bevy::prelude::*;
use ld60_devices::{ChipSpec, RomKind};
use ld60_levels::Level;

use super::job::Job;
use super::levels::Levels;
use crate::audio::{Play, Sound};
use crate::device::DeviceName;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;
use crate::locale::{Localized, Message};
use crate::parts::paper::{Binding, Paper, READING_ZOOM};
use crate::parts::screen::UiCanvas;
use crate::player::focus::{Focus, FocusTarget, View};

const SHEET_SIZE: Vec2 = Vec2::new(0.16, 0.22);
const CANVAS_SIZE: UVec2 = UVec2::new(640, 880);
pub(super) const BOARD_SIZE: Vec3 = Vec3::new(0.18, 0.005, 0.26);
const SHEET_GAP: f32 = 0.0003;
/// How far the top edge of the sheet reaches under the clip.
const SHEET_UNDER_CLIP: f32 = 0.01;
/// The jaw of the clip, pressing the sheet onto the board.
const JAW: Vec3 = Vec3::new(0.085, 0.0025, 0.02);
/// The spring housing behind the jaw.
const SPRING: Vec3 = Vec3::new(0.06, 0.009, 0.008);
const BOARD: Color = Color::srgb_u8(0x7a, 0x5a, 0x3c);
const CLIP: Color = Color::srgb_u8(0xa8, 0xac, 0xb0);
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
                (TicketStatus::update, TicketPays::update)
                    .run_if(resource_exists_and_changed::<Job>),
                Landing::animate,
            )
                .chain(),
        );
    }
}

/// The job ticket for a device on the bench, on a clipboard.
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
    ) {
        let Ok(ticket) = tickets.get(add.entity) else {
            return;
        };
        let level = &ticket.level;
        let canvas = UiCanvas::spawn_print(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let page = Paper::spawn_page(
            &mut commands,
            &canvas,
            (1, 1),
            Message::new("ticket-footer"),
            |page| Self::write(page, level),
        );
        let board = Block::new(BOARD_SIZE);
        let spring = Block::new(SPRING)
            .on_top_of(&board)
            .translated(Vec3::Z * (SPRING.z - BOARD_SIZE.z) / 2.0);
        let jaw = Block::new(JAW).on_top_of(&board).translated(Vec3::new(
            0.0,
            SHEET_GAP,
            (JAW.z - BOARD_SIZE.z) / 2.0,
        ));
        let sheet_center = Vec3::new(
            0.0,
            board.max().y + SHEET_GAP,
            jaw.max().z - SHEET_UNDER_CLIP + SHEET_SIZE.y / 2.0,
        );
        let mut sheet = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|clipboard| {
            clipboard.spawn((Shape::cuboid(&board, BOARD), Finish::Wood));
            sheet = clipboard
                .spawn(Paper::sheet(
                    &canvas,
                    ("ticket", &level.id),
                    SHEET_SIZE,
                    sheet_center,
                    &mut meshes,
                ))
                .id();
            for clip in [spring, jaw] {
                clipboard.spawn((Shape::cuboid(&clip, CLIP), Finish::Plated));
            }
        });
        commands.entity(add.entity).insert((
            Paper::new(sheet, SHEET_SIZE, vec![page]),
            Binding::clip(),
            FocusTarget {
                prompt: "click-ticket",
                focus: Focus::Paper,
                view: View::Local(Paper::reading_view(sheet_center)),
            },
            READING_ZOOM,
        ));
    }

    /// The job number, as the ticket and the receipt print it.
    pub(super) fn job(level: &Level) -> Localized {
        Localized::verbatim(level.id.to_uppercase())
    }

    fn row(label: &'static str, value: Localized) -> impl Bundle {
        Paper::row([
            (Message::new(label).into(), LABEL_WIDTH),
            (value, VALUE_WIDTH),
        ])
    }

    fn write(page: &mut ChildSpawnerCommands, level: &Level) {
        let chip = match level.chip {
            ChipSpec::Calco(info) => Message::new("ticket-chip-calco")
                .with("revision", info.revision.to_string())
                .with("date-code", info.date_code.to_string()),
        };
        let rom = match (level.rom.kind, level.rom.blank_available) {
            (RomKind::Eprom, true) => "ticket-rom-blank",
            (RomKind::Eprom, false) => "ticket-rom-no-blank",
            (RomKind::Mask, _) => "ticket-rom-mask",
        };

        page.spawn(Paper::title(Message::new("ticket-title")));
        page.spawn(Paper::faded(Message::new("ticket-subtitle")));
        for (label, value) in [
            ("ticket-job", Self::job(level)),
            (
                "ticket-customer",
                Levels::text(level, Level::CUSTOMER).into(),
            ),
            ("ticket-device", level.device.name().into()),
            ("ticket-serial", Localized::verbatim(level.serial())),
            ("ticket-board", Localized::verbatim(level.board().name())),
            ("ticket-chip", chip.into()),
            ("ticket-rom", Message::new(rom).into()),
        ] {
            page.spawn(Self::row(label, value));
        }
        page.spawn((
            TicketPays,
            Self::row("ticket-reward", TicketPays::text(level.reward)),
        ));
        page.spawn(Paper::heading(Message::new("ticket-complaint")));
        page.spawn(Paper::text(
            Message::new("paper-quote").with("text", Levels::text(level, Level::COMPLAINT)),
        ));
        page.spawn(Paper::heading(Message::new("ticket-remarks")));
        for remark in &level.remarks {
            page.spawn(Paper::text(
                Message::new("paper-bullet")
                    .with("text", Levels::text(level, Level::remark(remark))),
            ));
        }
        page.spawn(Paper::heading(Message::new("ticket-status")));
        page.spawn((TicketStatus, Paper::text(TicketStatus::text(None))));
        page.spawn(ShopStamp::bundle());
    }
}

/// The status line on the job ticket.
#[derive(Component, Debug)]
struct TicketStatus;

impl TicketStatus {
    fn text(job: Option<&Job>) -> Message {
        match job {
            Some(job) if job.stamped => Message::new("ticket-status-done"),
            Some(job) if job.flashed => Message::new("ticket-status-flashed"),
            Some(job) if job.returns > 0 => {
                Message::new("ticket-status-returned").with("tries", job.tries_left())
            }
            _ => Message::new("ticket-status-open"),
        }
    }

    fn update(
        job: Res<Job>,
        mut texts: Query<&mut Localized, With<TicketStatus>>,
        mut stamps: Query<(Entity, &mut Visibility, Has<Landing>), With<ShopStamp>>,
        mut commands: Commands,
    ) {
        for mut text in &mut texts {
            text.set_if_neq(Self::text(Some(&job)).into());
        }
        for (stamp, mut visibility, landing) in &mut stamps {
            if !job.stamped {
                *visibility = Visibility::Hidden;
                commands.entity(stamp).remove::<Landing>();
            } else if *visibility == Visibility::Hidden && !landing {
                commands.entity(stamp).insert(Landing::default());
            }
        }
    }
}

/// The row on the job ticket with what the customer pays, which drops with
/// each return.
#[derive(Component, Debug)]
struct TicketPays;

impl TicketPays {
    fn text(amount: u32) -> Localized {
        Message::new("receipt-amount").with("amount", amount).into()
    }

    /// The amount is the last text in the row.
    fn update(
        job: Res<Job>,
        rows: Query<Entity, With<TicketPays>>,
        children: Query<&Children>,
        mut texts: Query<&mut Localized>,
    ) {
        let pays = Self::text(job.reward());
        for row in &rows {
            let amount = children
                .iter_descendants(row)
                .filter(|&entity| texts.contains(entity))
                .last();
            if let Some(mut text) = amount.and_then(|amount| texts.get_mut(amount).ok()) {
                text.set_if_neq(pays.clone());
            }
        }
    }
}

/// The shop stamp coming down onto the ticket.
///
/// It waits for the camera to reach the ticket, then drops from above and
/// thunks when it hits the paper.
#[derive(Component, Debug, Default)]
pub(super) struct Landing {
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
        let ink = |size: f32, text: Localized| {
            (
                text,
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
                    ink(20.0, Message::new("ticket-stamp-authorized").into()),
                    ink(64.0, Localized::verbatim("DSG")),
                    ink(22.0, Message::new("ticket-stamp-shop").into()),
                    (
                        Node {
                            width: percent(100),
                            height: px(2),
                            ..default()
                        },
                        BackgroundColor(STAMP_INK),
                    ),
                    ink(16.0, Message::new("ticket-stamp-care").into()),
                ],
            )],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::Locale;
    use crate::workshop::levels::Levels;

    #[test]
    fn every_page_formats() {
        let locale = Locale::on_disk();
        for source in Levels::on_disk().levels() {
            let level = source.load().unwrap();
            locale.assert_formats(Localized::spawned_by(|page| {
                JobTicket::write(page, &level);
            }));
        }
        locale.assert_formats([Message::new("ticket-footer")]);
    }

    #[test]
    fn pays_drops_with_each_return() {
        let mut world = World::new();
        let mut job = Job::load(&Levels::on_disk(), 0).unwrap().unwrap();
        let full = job.level.reward;
        job.returns = 1;
        let cut = job.reward();
        assert!(cut < full);
        world.insert_resource(job);
        let amount = world.spawn(TicketPays::text(full)).id();
        world.spawn(TicketPays).add_children(&[amount]);
        world.run_system_cached(TicketPays::update).unwrap();
        assert_eq!(
            *world.get::<Localized>(amount).unwrap(),
            TicketPays::text(cut)
        );
    }

    #[test]
    fn every_status_formats() {
        let mut job = Job::load(&Levels::on_disk(), 0).unwrap().unwrap();
        let mut statuses = vec![TicketStatus::text(None), TicketStatus::text(Some(&job))];
        job.returns = 1;
        statuses.push(TicketStatus::text(Some(&job)));
        job.flashed = true;
        statuses.push(TicketStatus::text(Some(&job)));
        job.stamped = true;
        statuses.push(TicketStatus::text(Some(&job)));
        Locale::on_disk().assert_formats(statuses);
    }
}
