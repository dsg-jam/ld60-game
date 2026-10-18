use std::error::Error;
use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{input::keyboard::Key, prelude::*};
use ld60_devices::{AnyDevice, ChipSpec, DeviceKind};
use ld60_levels::{Level, LevelError, LevelSource};

use super::emulator::Emulator;
use super::{DeviceButton, Holdable};
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::parts::keypad::KeyBinding::{Named, Typed};
use crate::parts::keypad::{KeyDef, Keypad, KeypadStyle};
use crate::parts::lcd;
use crate::parts::screen::UiCanvas;

/// Dark frame around the LCD.
const LCD_BORDER: f32 = 0.003;
const WINDOW_HEIGHT: f32 = 0.0015;
/// Keeps the LCD from fighting with the window top.
const LCD_GAP: f32 = 0.0001;
/// Keeps the sticker from fighting with the body bottom.
const STICKER_GAP: f32 = 0.0002;
const STICKER_CANVAS: UVec2 = UVec2::new(384, 224);
const STICKER_FONT_SIZE: f32 = 26.0;
const STICKER_PAPER: Color = Color::srgb_u8(0xf2, 0xea, 0xd8);
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
const SUMMA6_ROWS: &[&[KeyDef<char>]] = &[
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

const TALLY2_ROWS: &[&[KeyDef<char>]] = &[&[
    KeyDef::unit("C", 'C', &[Typed("c"), Typed("C"), Named(Key::Delete)]),
    KeyDef::wide("+", '+', &[Typed("+"), Named(Key::Space)], 2.0),
]];

const SUMMA6: Model = Model {
    name: "SUMMA 6 ADDING MACHINE",
    body_size: Vec3::new(0.09, 0.014, 0.15),
    body: Color::srgb_u8(0x3d, 0x44, 0x50),
    lcd_width: 0.07,
    window_spot: Vec2::new(0.0, -0.6),
    rows: SUMMA6_ROWS,
    keypad_spot: Vec2::new(0.0, 0.3),
    sticker_size: Vec2::new(0.06, 0.035),
    sticker_spot: Vec2::new(0.0, 0.2),
};

const TALLY2: Model = Model {
    name: "TALLY 2 COUNTER",
    body_size: Vec3::new(0.065, 0.02, 0.085),
    body: Color::srgb_u8(0x8a, 0x3b, 0x2e),
    lcd_width: 0.03,
    window_spot: Vec2::new(0.0, -0.45),
    rows: TALLY2_ROWS,
    keypad_spot: Vec2::new(0.0, 0.45),
    sticker_size: Vec2::new(0.05, 0.029),
    sticker_spot: Vec2::ZERO,
};

/// Shape and keys of one device kind.
struct Model {
    /// Printed on the sticker.
    name: &'static str,
    body_size: Vec3,
    body: Color,
    /// Width of the LCD. Its height follows the canvas.
    lcd_width: f32,
    /// Where the display window sits on the body, see [`Block::top_spot`].
    window_spot: Vec2,
    rows: &'static [&'static [KeyDef<char>]],
    /// Where the keys sit on the body, see [`Block::top_spot`].
    keypad_spot: Vec2,
    sticker_size: Vec2,
    /// Where the sticker sits on the underside, see [`Block::top_spot`].
    sticker_spot: Vec2,
}

impl Model {
    fn of(kind: DeviceKind) -> &'static Self {
        match kind {
            DeviceKind::Tally2 => &TALLY2,
            DeviceKind::Summa6 => &SUMMA6,
        }
    }
}

pub(super) struct HandheldPlugin;

impl Plugin for HandheldPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Handheld::on_add);
    }
}

/// A handheld device as it comes in for a job, resting on the ground. The
/// level decides the model.
#[derive(Component, Debug)]
#[require(Holdable)]
pub(crate) struct Handheld {
    level: &'static LevelSource,
}

impl Handheld {
    /// A unit as it comes in for `level`, with its faulty firmware.
    pub(crate) fn for_level(level: &'static LevelSource) -> Self {
        Self { level }
    }

    fn on_add(
        add: On<Add<Handheld>>,
        units: Query<&Handheld>,
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

        let model = Model::of(level.device);
        let digits = level.device.display_cells().count();
        let lcd_canvas = lcd::canvas_size(digits).as_vec2();
        let lcd_size = Vec2::new(
            model.lcd_width,
            model.lcd_width * lcd_canvas.y / lcd_canvas.x,
        );
        let body = Block::new(model.body_size);
        let window = Block::new((lcd_size + 2.0 * LCD_BORDER).extend(WINDOW_HEIGHT).xzy())
            .resting_on(&body, model.window_spot);
        let keypad = Keypad::new(model.rows, &STYLE, &body, model.keypad_spot);
        let labels = keypad.spawn_labels(&mut commands, &mut images, add.entity);
        let lcd = lcd::spawn(&mut commands, &mut images, add.entity, digits);
        let sticker = Self::spawn_sticker(&mut commands, &mut images, add.entity, model, &level);
        let sticker_spot = body
            .top_spot(model.sticker_spot)
            .with_y(body.min().y - STICKER_GAP);
        let unlit = |texture: Handle<Image>| StandardMaterial {
            base_color_texture: Some(texture),
            unlit: true,
            ..default()
        };

        let mut entity = commands.entity(add.entity);
        match Self::build_device(unit.level, &level) {
            Ok(device) => {
                entity.insert(Emulator::power_on(device));
            }
            Err(err) => error!(
                error = &err as &dyn Error,
                "device for {} does not build", level.id
            ),
        }
        entity.with_children(|device| {
            device.spawn(Shape::cuboid(&body, model.body));
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
                Mesh3d(meshes.add(Rectangle::from_size(model.sticker_size))),
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

    fn sticker_text(model: &Model, level: &Level) -> String {
        let chip = match level.chip {
            ChipSpec::Calco(info) => format!("CALCO REV {:?} {}", info.revision, info.date_code),
        };
        format!(
            "{}\nS/N {}\n{chip}\nDSG ELECTRONICS",
            model.name, level.serial
        )
    }

    fn spawn_sticker(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        owner: Entity,
        model: &Model,
        level: &Level,
    ) -> Handle<Image> {
        let canvas = UiCanvas::spawn(commands, images, STICKER_CANVAS, owner);
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
            canvas.target(),
            children![(
                Text::new(Self::sticker_text(model, level)),
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
