use bevy::prelude::*;
use ld60_devices::DeviceKind;
use ld60_game_core::audio::{PlayAt, Sound};
use ld60_game_core::parts::lcd::{self, LcdDisplay};
use ld60_game_core::parts::screen::UiCanvas;
use ld60_game_core::parts::sticker::{Sticker, StickerMaterial};
use ld60_game_run::Run;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use ld60_locale::{Localized, Message};

use super::printer::Printing;
use super::receipt::Receipt;

/// Between the shop manuals and the stamp, where the player sees it.
pub(super) const SPOT: Vec2 = Vec2::new(-0.03, -0.72);
pub(super) const YAW: f32 = 0.08;
pub(super) const BOX_SIZE: Vec3 = Vec3::new(0.13, 0.045, 0.09);
const DIGITS: usize = 4;
const LID: f32 = 0.006;
const LID_OVERHANG: f32 = 0.002;
/// The housing of the display, standing on the back of the lid.
const PANEL: Vec3 = Vec3::new(0.088, 0.036, 0.014);
/// How far the panel leans back from upright, in radians.
const PANEL_TILT: f32 = 0.7;
/// How far behind the middle of the lid the panel stands.
const PANEL_BACK: f32 = 0.018;
const LCD_WIDTH: f32 = 0.074;
/// Keeps the LCD and the label from fighting with what they lie on.
const GAP: f32 = 0.0001;
const LABEL_SIZE: Vec2 = Vec2::new(0.04, 0.014);
const LABEL_CANVAS: UVec2 = UVec2::new(272, 96);
const LABEL_FONT_SIZE: f32 = 56.0;
const LABEL_PAPER: Color = Color::srgb_u8(0xf2, 0xea, 0xd8);
const LABEL_INK: Color = Color::srgb_u8(0x2a, 0x26, 0x24);
const LABEL_WEAR: f32 = 0.6;

/// The front of the cash drawer, set into the box, and its handle.
const DRAWER: Vec3 = Vec3::new(0.114, 0.026, 0.002);
const DRAWER_LOW: f32 = 0.004;
const HANDLE: Vec3 = Vec3::new(0.04, 0.004, 0.006);
const COIN_SLOT: Vec3 = Vec3::new(0.03, 0.001, 0.004);
const LOCK: Vec3 = Vec3::new(0.009, 0.002, 0.009);

const PAINT: Color = Color::srgb_u8(0x3e, 0x5a, 0x4c);
const DRAWER_PAINT: Color = Color::srgb_u8(0x2e, 0x44, 0x38);
const METAL: Color = Color::srgb_u8(0xb8, 0xbc, 0xc0);
const BRASS: Color = Color::srgb_u8(0xb0, 0x8a, 0x40);
const SLOT_DARK: Color = Color::srgb_u8(0x12, 0x12, 0x14);
const WINDOW: Color = Color::srgb_u8(0x26, 0x2a, 0x30);

/// Seconds between two counts of the display.
const TICK: f32 = 0.06;
/// Seconds the display waits before it counts a change, once any receipt
/// is out.
const WAIT: f32 = 0.6;
/// The display counts the change in at most this many steps.
const STEPS: u32 = 12;

pub(crate) struct TillPlugin;

impl Plugin for TillPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Till::on_add)
            .add_systems(Update, Till::count);
    }
}

/// The shop's cash box. Its display shows the money in the till and counts
/// up or down when the money changes.
#[derive(Component, Debug, Default)]
#[require(Transform, Visibility)]
pub(crate) struct Till {
    /// What the display shows, until it catches up with the till.
    shown: Option<u32>,
    /// How much each count adds or takes away.
    stride: u32,
    since: f32,
}

impl Till {
    /// The display can show up to this much.
    const MAX: u32 = 10_u32.pow(DIGITS as u32) - 1;

    /// How much each count adds or takes away on the way from `shown` to
    /// `till`.
    fn stride(shown: u32, till: u32) -> u32 {
        shown.abs_diff(till).div_ceil(STEPS).max(1)
    }

    /// The amount the display shows after one count of `stride` from
    /// `shown` towards `till`.
    fn step(shown: u32, till: u32, stride: u32) -> u32 {
        if shown < till {
            (shown + stride).min(till)
        } else {
            shown.saturating_sub(stride).max(till)
        }
    }

    fn digits(amount: u32) -> Vec<u8> {
        let amount = amount.min(Self::MAX);
        (0..DIGITS as u32)
            .rev()
            .map(|place| (amount / 10_u32.pow(place) % 10) as u8)
            .collect()
    }

    fn count(
        run: Res<Run>,
        time: Res<Time>,
        printing: Query<(), (With<Receipt>, With<Printing>)>,
        mut tills: Query<(&mut Till, &mut LcdDisplay, &GlobalTransform)>,
        mut commands: Commands,
    ) {
        // The money of a pickup goes in once its receipt is out.
        let waiting = !printing.is_empty();
        for (mut till, mut display, at) in &mut tills {
            let target = run.till();
            let Some(shown) = till.shown else {
                till.shown = Some(target);
                display.digits = Self::digits(target);
                continue;
            };
            if shown == target {
                till.since = 0.0;
                till.stride = 0;
                continue;
            }
            if till.stride == 0 || waiting {
                till.stride = Self::stride(shown, target);
                till.since = -WAIT;
                continue;
            }
            till.since += time.delta_secs();
            if till.since < TICK {
                continue;
            }
            till.since = 0.0;
            let next = Self::step(shown, target, till.stride);
            till.shown = Some(next);
            display.digits = Self::digits(next);
            commands.trigger(PlayAt(Sound::Coin, at.translation()));
        }
    }

    fn on_add(
        add: On<Add<Till>>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut stickers: ResMut<Assets<StickerMaterial>>,
    ) {
        // The Tally2 board wires no lamps, so the LCD shows only digits.
        let board = &DeviceKind::Tally2.boards()[0];
        let lcd = lcd::spawn(&mut commands, &mut images, add.entity, DIGITS, false, board);
        let canvas = lcd::canvas_size(DIGITS, false).as_vec2();
        let lcd_size = Vec2::new(LCD_WIDTH, LCD_WIDTH * canvas.y / canvas.x);
        let label = Self::spawn_label(&mut commands, &mut images, add.entity);
        let label_seed = Sticker::seed("till");

        let body = Block::new(BOX_SIZE.with_y(BOX_SIZE.y - LID));
        let lid = Block::new(BOX_SIZE.with_y(LID) + Vec3::new(2.0, 0.0, 2.0) * LID_OVERHANG)
            .on_top_of(&body);
        let panel = Block::new(PANEL).translated(Vec3::NEG_Y * PANEL.y / 2.0);
        // The panel rests on its bottom front edge.
        let tilt = Quat::from_rotation_x(-PANEL_TILT);
        let edge = lid.top_spot(Vec2::ZERO) + Vec3::NEG_Z * PANEL_BACK;
        let panel_at = edge + tilt * Vec3::new(0.0, PANEL.y / 2.0, -PANEL.z / 2.0);
        let drawer = Block::new(DRAWER).translated(Vec3::new(
            0.0,
            DRAWER_LOW,
            (BOX_SIZE.z + DRAWER.z) / 2.0,
        ));
        let handle = Block::new(HANDLE).translated(Vec3::new(
            0.0,
            DRAWER_LOW + 0.003,
            drawer.max().z + HANDLE.z / 2.0,
        ));
        let on_lid = |size: Vec3, x: f32, z: f32| {
            Block::new(size)
                .on_top_of(&lid)
                .translated(Vec3::new(x, 0.0, z))
        };
        let coin_slot = on_lid(COIN_SLOT, -0.03, 0.03);
        let lock = on_lid(LOCK, 0.045, 0.03);
        // The label sits on the drawer, above the handle.
        let label_at = Vec3::new(0.0, DRAWER_LOW + DRAWER.y * 0.62, drawer.max().z + GAP);

        commands
            .entity(add.entity)
            .insert(LcdDisplay {
                digits: Self::digits(0),
                ..default()
            })
            .with_children(|till| {
                till.spawn((Shape::cuboid(&body, PAINT), Finish::Paint));
                till.spawn((Shape::cuboid(&lid, PAINT), Finish::Paint));
                till.spawn((Shape::cuboid(&drawer, DRAWER_PAINT), Finish::Paint));
                till.spawn((Shape::cuboid(&handle, METAL), Finish::Plated));
                till.spawn((Shape::cuboid(&coin_slot, SLOT_DARK), Finish::Plastic));
                till.spawn((Shape::cylinder(&lock, BRASS), Finish::Metal));
                till.spawn((
                    Transform::from_translation(panel_at).with_rotation(tilt),
                    Visibility::default(),
                ))
                .with_children(|panel_frame| {
                    panel_frame.spawn((Shape::cuboid(&panel, WINDOW), Finish::Plastic));
                    panel_frame.spawn((
                        Mesh3d(meshes.add(Rectangle::from_size(lcd_size))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color_texture: Some(lcd),
                            unlit: true,
                            ..default()
                        })),
                        Transform::from_translation(Vec3::Z * (PANEL.z / 2.0 + GAP)),
                    ));
                });
                till.spawn((
                    Mesh3d(meshes.add(Sticker::mesh(LABEL_SIZE, label_seed))),
                    MeshMaterial3d(
                        stickers.add(Sticker::material(label, LABEL_SIZE, label_seed, LABEL_WEAR)),
                    ),
                    Transform::from_translation(label_at),
                ));
            });
    }

    fn spawn_label(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        owner: Entity,
    ) -> Handle<Image> {
        let canvas = UiCanvas::spawn(commands, images, LABEL_CANVAS, owner);
        commands.spawn((
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(LABEL_PAPER),
            canvas.target(),
            children![(
                Localized::from(Message::new("till-label")),
                TextFont {
                    font_size: FontSize::Px(LABEL_FONT_SIZE),
                    ..default()
                },
                TextColor(LABEL_INK),
            )],
        ));
        canvas.image
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_display_counts_to_the_till() {
        for (from, to) in [(0, 5), (0, 400), (120, 6), (7, 0)] {
            let mut shown = from;
            let stride = Till::stride(from, to);
            let mut steps = 0;
            while shown != to {
                shown = Till::step(shown, to, stride);
                steps += 1;
            }
            assert!((1..=STEPS).contains(&steps), "{from} to {to}: {steps}");
        }
    }

    #[test]
    fn the_display_shows_four_digits() {
        assert_eq!(Till::digits(42), [0, 0, 4, 2]);
        assert_eq!(Till::digits(123_456), [9, 9, 9, 9]);
    }
}
