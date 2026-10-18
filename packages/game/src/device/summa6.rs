use std::error::Error;
use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{input::keyboard::Key, prelude::*};
use ld60_devices::{AnyDevice, ChipSpec};
use ld60_levels::{Level, LevelError, LevelSource};

use super::emulator::Emulator;
use super::{DeviceButton, Holdable};
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::parts::keypad::KeyBinding::{Named, Typed};
use crate::parts::keypad::{KeyDef, Keypad, KeypadStyle};
use crate::parts::lcd;
use crate::parts::screen::UiCanvas;

const BODY_SIZE: Vec3 = Vec3::new(0.09, 0.014, 0.15);
/// Width of the LCD. Its height follows the canvas.
const LCD_WIDTH: f32 = 0.07;
/// Dark frame around the LCD.
const LCD_BORDER: f32 = 0.003;
const WINDOW_HEIGHT: f32 = 0.0015;
/// Keeps the LCD from fighting with the window top.
const LCD_GAP: f32 = 0.0001;
const DIGITS: usize = 6;
/// RAM noise at power on. Fixed until jobs provide one.
const POWER_ON_SEED: u64 = 1;
/// Where the display window sits on the body, see [`Block::top_spot`].
const WINDOW_SPOT: Vec2 = Vec2::new(0.0, -0.6);
/// Where the keys sit on the body, see [`Block::top_spot`].
const KEYPAD_SPOT: Vec2 = Vec2::new(0.0, 0.3);

/// The label on the underside.
const STICKER_SIZE: Vec2 = Vec2::new(0.06, 0.035);
/// Where the sticker sits on the underside, see [`Block::top_spot`].
const STICKER_SPOT: Vec2 = Vec2::new(0.0, 0.2);
/// Keeps the sticker from fighting with the body bottom.
const STICKER_GAP: f32 = 0.0002;
const STICKER_CANVAS: UVec2 = UVec2::new(384, 224);
const STICKER_FONT_SIZE: f32 = 26.0;
const STICKER_PAPER: Color = Color::srgb_u8(0xf2, 0xea, 0xd8);
/// Placeholder until the unit comes from a job.
const SERIAL: &str = "6A-04417";

const BODY: Color = Color::srgb_u8(0x3d, 0x44, 0x50);
const WINDOW: Color = Color::srgb_u8(0x26, 0x2a, 0x30);

const STYLE: KeypadStyle = KeypadStyle {
    pitch: 0.019,
    key_size: Vec3::new(0.016, 0.003, 0.014),
    travel: 0.0015,
    label_cell: 128.0,
    font_size: 64.0,
    word_font_size: 36.0,
    ink: Color::srgb_u8(0x2a, 0x26, 0x24),
    cap: Color::srgb_u8(0xe8, 0xe0, 0xcc),
};

/// Rows from the back to the front.
const ROWS: &[&[KeyDef<char>]] = &[
    &[
        KeyDef::unit("7", '7', &[Typed("7")]),
        KeyDef::unit("8", '8', &[Typed("8")]),
        KeyDef::unit("9", '9', &[Typed("9")]),
        KeyDef::unit("C", 'C', &[Typed("c"), Typed("C"), Named(Key::Delete)]),
    ],
    &[
        KeyDef::unit("4", '4', &[Typed("4")]),
        KeyDef::unit("5", '5', &[Typed("5")]),
        KeyDef::unit("6", '6', &[Typed("6")]),
        KeyDef::unit("-", '-', &[Typed("-")]),
    ],
    &[
        KeyDef::unit("1", '1', &[Typed("1")]),
        KeyDef::unit("2", '2', &[Typed("2")]),
        KeyDef::unit("3", '3', &[Typed("3")]),
        KeyDef::unit("+", '+', &[Typed("+")]),
    ],
    &[
        KeyDef::wide("0", '0', &[Typed("0")], 2.0),
        KeyDef::wide("=", '=', &[Typed("="), Named(Key::Enter)], 2.0),
    ],
];

pub(super) struct Summa6Plugin;

impl Plugin for Summa6Plugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Summa6::on_add);
    }
}

/// A 6-digit adding machine, resting on the ground.
#[derive(Component, Debug)]
#[require(Holdable)]
pub(crate) struct Summa6 {
    level: &'static LevelSource,
}

impl Summa6 {
    /// A unit as it comes in for `level`, with its faulty firmware.
    pub(crate) fn for_level(level: &'static LevelSource) -> Self {
        Self { level }
    }

    fn on_add(
        add: On<Add<Summa6>>,
        units: Query<&Summa6>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok(unit) = units.get(add.entity) else {
            return;
        };
        let level = match unit.level.load() {
            Ok(level) => level,
            Err(err) => {
                error!(
                    error = &err as &dyn Error,
                    "level {} does not load", unit.level.path
                );
                return;
            }
        };

        let lcd_canvas = lcd::canvas_size(DIGITS).as_vec2();
        let lcd_size = Vec2::new(LCD_WIDTH, LCD_WIDTH * lcd_canvas.y / lcd_canvas.x);
        let body = Block::new(BODY_SIZE);
        let window = Block::new((lcd_size + 2.0 * LCD_BORDER).extend(WINDOW_HEIGHT).xzy())
            .resting_on(&body, WINDOW_SPOT);
        let keypad = Keypad::new(ROWS, &STYLE, &body, KEYPAD_SPOT);
        let labels = keypad.spawn_labels(&mut commands, &mut images);
        let lcd = lcd::spawn(&mut commands, &mut images, add.entity, DIGITS);
        let sticker = Self::spawn_sticker(&mut commands, &mut images, &level.chip);
        let sticker_spot = body
            .top_spot(STICKER_SPOT)
            .with_y(body.min().y - STICKER_GAP);
        let unlit = |texture: Handle<Image>| StandardMaterial {
            base_color_texture: Some(texture),
            unlit: true,
            ..default()
        };

        let mut entity = commands.entity(add.entity);
        match Self::build_device(unit.level, &level) {
            Ok(device) => {
                entity.insert(Emulator::power_on(device, POWER_ON_SEED));
            }
            Err(err) => error!(
                error = &err as &dyn Error,
                "device for {} does not build", level.id
            ),
        }
        entity.with_children(|device| {
            device.spawn(Shape::cuboid(&body, BODY));
            device.spawn(Shape::cuboid(&window, WINDOW));
            device.spawn((
                Mesh3d(meshes.add(Rectangle::from_size(lcd_size))),
                MeshMaterial3d(materials.add(unlit(lcd))),
                // Faces up, with the top of the canvas away from the player.
                Transform::from_translation(window.top_spot(Vec2::ZERO) + Vec3::Y * LCD_GAP)
                    .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
            ));
            keypad.spawn_keys(device, &labels, false, |key| DeviceButton::new(key.key));
            device.spawn((
                Mesh3d(meshes.add(Rectangle::from_size(STICKER_SIZE))),
                MeshMaterial3d(materials.add(unlit(sticker))),
                // Faces down, readable when the device is flipped over
                // sideways.
                Transform::from_translation(sticker_spot)
                    .with_rotation(Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_z(PI)),
            ));
        });
    }

    fn build_device(source: &LevelSource, level: &Level) -> Result<AnyDevice, LevelError> {
        let rom = ld60_levels::initial_rom(level, source.buggy)?;
        ld60_levels::device(level, rom)
    }

    fn sticker_text(chip: &ChipSpec) -> String {
        let chip = match chip {
            ChipSpec::Calco(info) => format!("CALCO REV {:?} {}", info.revision, info.date_code),
        };
        format!("SUMMA 6 ADDING MACHINE\nS/N {SERIAL}\n{chip}\nDSG ELECTRONICS")
    }

    fn spawn_sticker(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        chip: &ChipSpec,
    ) -> Handle<Image> {
        let canvas = UiCanvas::spawn(commands, images, STICKER_CANVAS);
        commands.spawn((
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                padding: px(16).all(),
                ..default()
            },
            BackgroundColor(STICKER_PAPER),
            UiTargetCamera(canvas.camera),
            children![(
                Text::new(Self::sticker_text(chip)),
                TextFont {
                    font_size: FontSize::Px(STICKER_FONT_SIZE),
                    ..default()
                },
                TextColor(STYLE.ink),
            )],
        ));
        canvas.image
    }
}
