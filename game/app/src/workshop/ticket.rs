use bevy::prelude::*;
use ld60_devices::{ChipSpec, RomKind};
use ld60_game_core::parts::paper::{Binding, Flutter, Paper};
use ld60_game_core::parts::screen::UiCanvas;
use ld60_game_core::player::focus::{Focus, FocusTarget, View};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use ld60_levels::Level;
use ld60_locale::{Localized, Message};

use ld60_game_device::DeviceName;
use ld60_game_run::job::Job;
use ld60_game_run::levels::Levels;
use ld60_game_run::regulars::Visit;

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
/// About the middle of the stamp on the canvas, where the rubber stamp
/// presses.
const STAMP_MIDDLE: Vec2 = Vec2::new(440.0, 615.0);
const STAMP_INK: Color = Color::srgba(0.72, 0.12, 0.1, 0.8);
const LABEL_WIDTH: f32 = 120.0;
const VALUE_WIDTH: f32 = 440.0;

pub(crate) struct TicketPlugin;

impl Plugin for TicketPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(JobTicket::on_add)
            .add_observer(Clipboard::on_add)
            .add_systems(
                Update,
                (TicketStatus::update, TicketPays::update)
                    .run_if(resource_exists_and_changed::<Job>),
            );
    }
}

/// The clipboard on the counter that holds the [`JobTicket`].
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Clipboard;

impl Clipboard {
    fn board_and_jaw() -> (Block, Block) {
        let board = Block::new(BOARD_SIZE);
        let jaw = Block::new(JAW).on_top_of(&board).translated(Vec3::new(
            0.0,
            SHEET_GAP,
            (JAW.z - BOARD_SIZE.z) / 2.0,
        ));
        (board, jaw)
    }

    fn on_add(add: On<Add<Clipboard>>, mut commands: Commands) {
        let (board, jaw) = Self::board_and_jaw();
        let spring = Block::new(SPRING)
            .on_top_of(&board)
            .translated(Vec3::Z * (SPRING.z - BOARD_SIZE.z) / 2.0);
        commands.entity(add.entity).with_children(|clipboard| {
            clipboard.spawn((Shape::cuboid(&board, BOARD), Finish::Wood));
            for clip in [spring, jaw] {
                clipboard.spawn((Shape::cuboid(&clip, CLIP), Finish::Plated));
            }
        });
    }
}

/// The job ticket for a device on the bench, a sheet on the [`Clipboard`].
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct JobTicket {
    level: Level,
    /// The regular who brought the device, if a regular did.
    visit: Option<Visit>,
}

impl JobTicket {
    pub(crate) fn new(level: &Level, visit: Option<Visit>) -> Self {
        Self {
            level: level.clone(),
            visit,
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
        let (level, visit) = (&ticket.level, ticket.visit);
        let canvas = UiCanvas::spawn_print(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let page = Paper::spawn_page(
            &mut commands,
            &canvas,
            (1, 1),
            Message::new("ticket-footer"),
            |page| Self::write(page, level, visit),
        );
        let sheet_center = Self::sheet_center();
        let sheet = commands
            .spawn((
                Paper::sheet(
                    &canvas,
                    ("ticket", &level.id),
                    SHEET_SIZE,
                    sheet_center,
                    &mut meshes,
                ),
                Flutter::default(),
                ChildOf(add.entity),
            ))
            .id();
        commands.entity(add.entity).insert((
            Paper::new(sheet, SHEET_SIZE, vec![page]),
            Binding::clip(),
            FocusTarget {
                prompt: "click-ticket",
                focus: Focus::Paper,
                view: View::Local(Paper::reading_view(sheet_center)),
            },
        ));
    }

    pub(super) const SHEET_SIZE: Vec2 = SHEET_SIZE;

    /// The middle of the sheet, which lies under the clip of the
    /// [`Clipboard`].
    pub(super) fn sheet_center() -> Vec3 {
        let (board, jaw) = Clipboard::board_and_jaw();
        Vec3::new(
            0.0,
            board.max().y + SHEET_GAP,
            jaw.max().z - SHEET_UNDER_CLIP + SHEET_SIZE.y / 2.0,
        )
    }

    /// Where the shop stamp is printed, in the space of the ticket. Down
    /// the page is towards the player.
    pub(super) fn stamp_spot() -> Vec3 {
        let along = STAMP_MIDDLE / CANVAS_SIZE.as_vec2() - 0.5;
        Self::sheet_center() + Vec3::new(along.x * SHEET_SIZE.x, 0.0, along.y * SHEET_SIZE.y)
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

    fn write(page: &mut ChildSpawnerCommands, level: &Level, visit: Option<Visit>) {
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
        let complaint = Levels::text(level, Level::COMPLAINT);
        let complaint = match visit {
            Some(visit) => Message::new("ticket-complaint-visit")
                .with("story", visit.story())
                .with("complaint", complaint),
            None => complaint,
        };
        page.spawn(Paper::text(
            Message::new("paper-quote").with("text", complaint),
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
        mut stamps: Query<&mut Visibility, With<ShopStamp>>,
    ) {
        for mut text in &mut texts {
            text.set_if_neq(Self::text(Some(&job)).into());
        }
        for mut visibility in &mut stamps {
            visibility.set_if_neq(if job.stamped {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
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
    use ld60_game_run::levels::Levels;
    use ld60_game_run::regulars::Regular;
    use ld60_locale::Locale;

    use super::*;

    #[test]
    fn every_page_formats() {
        let locale = Locale::on_disk();
        for source in Levels::on_disk().levels() {
            let level = source.load().unwrap();
            locale.assert_formats(Localized::spawned_by(|page| {
                JobTicket::write(page, &level, None);
            }));
        }
        let level = Levels::on_disk().levels()[0].load().unwrap();
        let visit = Visit {
            regular: Regular::Vosh,
            number: 2,
        };
        locale.assert_formats(Localized::spawned_by(|page| {
            JobTicket::write(page, &level, Some(visit));
        }));
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
