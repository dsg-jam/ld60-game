use std::error::Error;
use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{input::keyboard::Key, prelude::*};
use ld60_devices::{AnyDevice, ChipSpec, DeviceKind};
use ld60_game_core::parts::keypad::KeyBinding::{Named, Typed};
use ld60_game_core::parts::keypad::{KeyDef, Keypad, KeypadStyle};
use ld60_game_core::parts::lcd;
use ld60_game_core::parts::marking::Marking;
use ld60_game_core::parts::pcb::PcbMaterial;
use ld60_game_core::parts::screen::UiCanvas;
use ld60_game_core::parts::sticker::{Sticker, StickerMaterial};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use ld60_levels::{Level, LevelError};
use ld60_locale::{Locale, Localized, Message};

use super::board::{BoardLayout, BoardSpots, CHIP_BLACK, CHIP_INK, SilkWords};
use super::casing::{COVER, Casing, CasingSpots, WALL};
use super::emulator::Emulator;
use super::silhouette::Silhouette;
use super::{DeviceButton, DeviceName, Holdable};

/// Dark frame around the LCD.
const LCD_BORDER: f32 = 0.003;
/// Tinned copper inside the holes of a board.
const PLATING: Color = Color::srgb_u8(0xd8, 0xd6, 0xcc);
const WINDOW_HEIGHT: f32 = 0.0015;
/// Keeps the LCD from fighting with the window top.
const LCD_GAP: f32 = 0.0001;
/// Keeps the sticker from fighting with the body bottom.
const STICKER_GAP: f32 = 0.0002;
const STICKER_CANVAS: UVec2 = UVec2::new(384, 224);
const STICKER_FONT_SIZE: f32 = 26.0;
const STICKER_PAPER: Color = Color::srgb_u8(0xf2, 0xea, 0xd8);
const STICKER_WEAR: f32 = 0.8;
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
    shape: (0.0, 0.0),
    row_gaps: &[],
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

const TALLY1_ROWS: &[&[KeyDef<char>]] = &[&[KeyDef::wide(
    "+",
    '+',
    &[Typed("+"), Named(Key::Space)],
    2.0,
)]];

const TALLY2_ROWS: &[&[KeyDef<char>]] = &[&[
    KeyDef::unit("C", 'C', &[Typed("c"), Typed("C"), Named(Key::Delete)]),
    KeyDef::wide("+", '+', &[Typed("+"), Named(Key::Space)], 2.0),
]];

const SUMMA6: Model = Model {
    body_size: Vec3::new(0.09, 0.014, 0.15),
    body: Color::srgb_u8(0x3d, 0x44, 0x50),
    lcd_width: 0.07,
    window_spot: Vec2::new(0.0, -0.6),
    rows: SUMMA6_ROWS,
    keypad_spot: Vec2::new(0.0, 0.3),
    sticker_size: Vec2::new(0.06, 0.035),
    sticker_spot: Vec2::new(0.0, 0.2),
    casing: CasingSpots {
        screws: &[
            Vec2::new(-0.8, -0.85),
            Vec2::new(0.8, -0.85),
            Vec2::new(-0.8, 0.85),
            Vec2::new(0.8, 0.85),
        ],
        header: Vec2::new(0.0, -0.1),
    },
    words: false,
};

const LOCK4: Model = Model {
    body_size: Vec3::new(0.075, 0.022, 0.16),
    body: Color::srgb_u8(0x6b, 0x6e, 0x5a),
    lcd_width: 0.06,
    window_spot: Vec2::new(0.0, -0.7),
    rows: LOCK4_ROWS,
    keypad_spot: Vec2::new(0.0, 0.25),
    sticker_size: Vec2::new(0.055, 0.032),
    sticker_spot: Vec2::new(0.0, 0.2),
    casing: CasingSpots {
        screws: &[
            Vec2::new(-0.75, -0.85),
            Vec2::new(0.75, -0.85),
            Vec2::new(0.0, 0.88),
        ],
        header: Vec2::new(0.0, -0.1),
    },
    words: true,
};

const LOCK4_ROWS: &[&[KeyDef<char>]] = &[
    &[
        KeyDef::unit("1", '1', &[Typed("1")]),
        KeyDef::unit("2", '2', &[Typed("2")]),
        KeyDef::unit("3", '3', &[Typed("3")]),
    ],
    &[
        KeyDef::unit("4", '4', &[Typed("4")]),
        KeyDef::unit("5", '5', &[Typed("5")]),
        KeyDef::unit("6", '6', &[Typed("6")]),
    ],
    &[
        KeyDef::unit("7", '7', &[Typed("7")]),
        KeyDef::unit("8", '8', &[Typed("8")]),
        KeyDef::unit("9", '9', &[Typed("9")]),
    ],
    &[
        KeyDef::unit("*", '*', &[Typed("*")]),
        KeyDef::unit("0", '0', &[Typed("0")]),
        KeyDef::unit("#", '#', &[Typed("#"), Named(Key::Enter)]),
    ],
    &[KeyDef::wide(
        "C",
        'C',
        &[Typed("c"), Typed("C"), Named(Key::Delete)],
        3.0,
    )],
];

const SCORE5_ROWS: &[&[KeyDef<char>]] = &[
    &[
        KeyDef::unit("1", '1', &[Typed("1")]),
        KeyDef::unit("2", '2', &[Typed("2")]),
        KeyDef::unit("3", '3', &[Typed("3")]),
        KeyDef::unit("C", 'C', &[Typed("c"), Typed("C"), Named(Key::Delete)]),
    ],
    &[
        KeyDef::unit("4", '4', &[Typed("4")]),
        KeyDef::unit("5", '5', &[Typed("5")]),
        KeyDef::unit("6", '6', &[Typed("6")]),
        KeyDef::unit("P", 'P', &[Typed("p"), Typed("P")]),
    ],
    &[
        KeyDef::unit("7", '7', &[Typed("7")]),
        KeyDef::unit("8", '8', &[Typed("8")]),
        KeyDef::unit("9", '9', &[Typed("9")]),
        KeyDef::unit("0", '0', &[Typed("0")]),
    ],
    &[
        KeyDef::wide("H", 'H', &[Typed("h"), Typed("H")], 2.0),
        KeyDef::wide("G", 'G', &[Typed("g"), Typed("G")], 2.0),
    ],
];

const SCORE5: Model = Model {
    body_size: Vec3::new(0.1, 0.02, 0.14),
    body: Color::srgb_u8(0x2e, 0x46, 0x6e),
    lcd_width: 0.08,
    window_spot: Vec2::new(0.0, -0.66),
    rows: SCORE5_ROWS,
    keypad_spot: Vec2::new(0.0, 0.3),
    sticker_size: Vec2::new(0.06, 0.035),
    sticker_spot: Vec2::new(0.0, 0.1),
    casing: CasingSpots {
        screws: &[
            Vec2::new(-0.82, -0.86),
            Vec2::new(0.82, -0.86),
            Vec2::new(-0.82, 0.86),
            Vec2::new(0.82, 0.86),
        ],
        header: Vec2::new(0.0, -0.1),
    },
    words: false,
};

const TALLY2: Model = Model {
    body_size: Vec3::new(0.065, 0.02, 0.085),
    body: Color::srgb_u8(0x8a, 0x3b, 0x2e),
    lcd_width: 0.03,
    window_spot: Vec2::new(0.0, -0.45),
    rows: TALLY2_ROWS,
    keypad_spot: Vec2::new(0.0, 0.45),
    sticker_size: Vec2::new(0.05, 0.029),
    sticker_spot: Vec2::ZERO,
    casing: CasingSpots {
        screws: &[Vec2::new(-0.82, -0.8), Vec2::new(0.82, 0.8)],
        header: Vec2::new(0.0, -0.1),
    },
    words: false,
};

const TALLY1: Model = Model {
    body_size: Vec3::new(0.055, 0.02, 0.07),
    body: Color::srgb_u8(0xb8, 0x8c, 0x2c),
    lcd_width: 0.018,
    window_spot: Vec2::new(0.0, -0.45),
    rows: TALLY1_ROWS,
    keypad_spot: Vec2::new(0.0, 0.45),
    sticker_size: Vec2::new(0.045, 0.026),
    sticker_spot: Vec2::ZERO,
    casing: CasingSpots {
        screws: &[Vec2::new(-0.8, -0.8), Vec2::new(0.8, 0.8)],
        header: Vec2::new(0.0, -0.1),
    },
    words: false,
};

/// Shape and keys of one device kind.
pub(super) struct Model {
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
    casing: CasingSpots,
    /// Whether the LCD prints the OPEN and SHUT lamps.
    words: bool,
}

impl Model {
    pub(super) fn of(kind: DeviceKind) -> &'static Self {
        match kind {
            DeviceKind::Tally2 => &TALLY2,
            DeviceKind::Summa6 => &SUMMA6,
            DeviceKind::Lock4 => &LOCK4,
            DeviceKind::Score5 => &SCORE5,
            DeviceKind::Tally1 => &TALLY1,
        }
    }

    /// The LCD for `digits` digits.
    fn lcd_size(&self, digits: usize) -> Vec2 {
        let canvas = lcd::canvas_size(digits, self.words).as_vec2();
        Vec2::new(self.lcd_width, self.lcd_width * canvas.y / canvas.x)
    }

    /// The display window around an LCD of `lcd_size`.
    fn window(&self, body: &Block, lcd_size: Vec2) -> Block {
        Block::new((lcd_size + 2.0 * LCD_BORDER).extend(WINDOW_HEIGHT).xzy())
            .resting_on(body, self.window_spot)
    }

    /// The body, the display window and the keys as seen from above, for a
    /// unit with `digits` digits.
    pub(super) fn silhouette(&self, digits: usize) -> Silhouette {
        let body = Block::new(self.body_size);
        let window = self.window(&body, self.lcd_size(digits));
        let keypad = Keypad::new(self.rows, &STYLE, &body, self.keypad_spot);
        let keys = keypad
            .keys()
            .iter()
            .map(|key| (Silhouette::top_view(&key.block), STYLE.cap));
        Silhouette::new(
            [
                (Silhouette::top_view(&body), self.body),
                (Silhouette::top_view(&window), WINDOW),
            ]
            .into_iter()
            .chain(keys)
            .collect(),
        )
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
    level: Level,
    /// The faulty firmware it comes in with.
    buggy: String,
}

#[cfg(test)]
impl Handheld {
    pub(crate) fn body_size(kind: DeviceKind) -> Vec3 {
        Model::of(kind).body_size
    }

    pub(crate) fn screw_count(kind: DeviceKind) -> usize {
        Model::of(kind).casing.screws.len()
    }
}

impl Handheld {
    /// A unit as it comes in for `level`, running `buggy`.
    pub(crate) fn for_job(level: &Level, buggy: &str) -> Self {
        Self {
            level: level.clone(),
            buggy: buggy.into(),
        }
    }

    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn on_add(
        add: On<Add<Handheld>>,
        units: Query<&Handheld>,
        mut holdables: Query<&mut Holdable>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut stickers: ResMut<Assets<StickerMaterial>>,
        mut pcbs: ResMut<Assets<PcbMaterial>>,
        locale: Res<Locale>,
    ) {
        let Ok(unit) = units.get(add.entity) else {
            return;
        };
        let level = &unit.level;
        let model = Model::of(level.device);
        let digits = level.device.digits();
        let lcd_size = model.lcd_size(digits);
        let body = Block::new(model.body_size);
        let window = model.window(&body, lcd_size);
        let keypad = Keypad::new(model.rows, &STYLE, &body, model.keypad_spot);
        let labels = keypad.spawn_labels(&mut commands, &mut images, add.entity);
        let lcd = lcd::spawn(
            &mut commands,
            &mut images,
            add.entity,
            digits,
            model.words,
            level.board(),
        );
        let sticker = Self::spawn_sticker(&mut commands, &mut images, add.entity, level);
        let sticker_seed = Sticker::seed(&level.serial());
        let sticker_spot = body
            .top_spot(model.sticker_spot)
            .with_y(body.min().y - STICKER_GAP);
        // The cover is the bottom of the body. Under it are the board and
        // the header, inside the cover.
        let shell = Block::new(model.body_size.with_y(model.body_size.y - COVER))
            .translated(Vec3::Y * COVER);
        let height = model.body_size.y + STYLE.key_size.y;
        if let Ok(mut holdable) = holdables.get_mut(add.entity) {
            holdable.pivot = Vec3::Y * height / 2.0;
        }
        let plate = Block::new(shell.size.with_y(COVER));
        let screws: Vec<_> = model
            .casing
            .screws
            .iter()
            .map(|&spot| plate.top_spot(spot).with_y(0.0))
            .collect();
        let spots = BoardSpots {
            header: model.casing.header,
            display: model.window_spot,
            screws: &screws,
        };
        // Boards of one revision come off the same artwork.
        let name = format!("PCB {}", level.board().name());
        let artwork = Sticker::seed(&format!("{:?} {name}", level.device));
        let board = BoardLayout::new(&shell, COVER, &spots, artwork);
        let faces = board.faces(
            &name,
            &SilkWords::new(&locale),
            &mut meshes,
            &mut images,
            &mut pcbs,
        );
        let marking = images.add(Self::chip_marking(level));
        let (plating, platings) = board.platings();
        let plating = (
            Mesh3d(meshes.add(plating)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: PLATING,
                metallic: 1.0,
                perceptual_roughness: 0.35,
                ..default()
            })),
        );
        let unlit = |texture: Handle<Image>| StandardMaterial {
            base_color_texture: Some(texture),
            unlit: true,
            ..default()
        };

        let mut entity = commands.entity(add.entity);
        match unit.build_device() {
            Ok(device) => {
                entity.insert(Emulator::power_on(device));
            }
            Err(err) => error!(
                error = &err as &dyn Error,
                "device for {} does not build", level.id
            ),
        }
        let mut casing = None;
        entity.with_children(|device| {
            for wall in Self::walls(&shell) {
                device.spawn((Shape::cuboid(&wall, model.body), Finish::Plastic));
            }
            device.spawn((Shape::cuboid(&window, WINDOW), Finish::Plastic));
            device.spawn((
                Mesh3d(meshes.add(Rectangle::from_size(lcd_size))),
                MeshMaterial3d(materials.add(unlit(lcd))),
                // Faces up, with the top of the canvas away from the player.
                Transform::from_translation(window.top_spot(Vec2::ZERO) + Vec3::Y * LCD_GAP)
                    .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
            ));
            keypad.spawn_keys(device, &labels, false, |key| DeviceButton::new(key.key));
            let sticker = (
                Mesh3d(meshes.add(Sticker::mesh(model.sticker_size, sticker_seed))),
                MeshMaterial3d(stickers.add(Sticker::material(
                    sticker,
                    model.sticker_size,
                    sticker_seed,
                    STICKER_WEAR,
                ))),
                // Faces down, readable when the device is flipped over
                // sideways.
                Transform::from_translation(sticker_spot)
                    .with_rotation(Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_z(PI)),
            );
            // The tube of a hole runs along Z.
            let along_y = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
            for center in platings {
                device.spawn((
                    plating.clone(),
                    Transform::from_translation(center).with_rotation(along_y),
                ));
            }
            casing = Some(Casing::spawn(
                device,
                &shell,
                model.body,
                &model.casing,
                sticker,
                &board,
                faces,
                marking.clone(),
                height,
            ));
        });
        if let Some(casing) = casing {
            entity.insert(casing);
        }
    }

    /// The front and the four sides of a hollow `shell`. The back cover
    /// closes it.
    fn walls(shell: &Block) -> [Block; 5] {
        let side = shell.size.with_y(shell.size.y - WALL);
        let x = Block::new(side.with_x(WALL)).translated(Vec3::Y * shell.min().y);
        let z = Block::new(side.with_x(shell.size.x - 2.0 * WALL).with_z(WALL))
            .translated(Vec3::Y * shell.min().y);
        let inset = |size: f32| size / 2.0 - WALL / 2.0;
        [
            Block::new(shell.size.with_y(WALL)).translated(Vec3::Y * (shell.max().y - WALL)),
            x.translated(Vec3::X * -inset(shell.size.x)),
            x.translated(Vec3::X * inset(shell.size.x)),
            z.translated(Vec3::Z * -inset(shell.size.z)),
            z.translated(Vec3::Z * inset(shell.size.z)),
        ]
    }

    fn build_device(&self) -> Result<AnyDevice, LevelError> {
        let rom = ld60_levels::initial_rom(&self.level, &self.buggy)?;
        ld60_levels::device(&self.level, rom)
    }

    /// The maker, revision and date code etched into the chip.
    fn chip_marking(level: &Level) -> Image {
        let (maker, revision, date_code) = match level.chip {
            ChipSpec::Calco(info) => ("CALCO", info.revision, info.date_code),
        };
        let code = format!("{revision} {date_code}");
        Marking::image(&[maker, &code], CHIP_INK, CHIP_BLACK)
    }

    fn sticker_text(level: &Level) -> Message {
        let chip = match level.chip {
            ChipSpec::Calco(info) => Message::new("device-chip-calco")
                .with("revision", info.revision.to_string())
                .with("date-code", info.date_code.to_string()),
        };
        Message::new("device-sticker")
            .with("name", level.device.plate())
            .with("serial", level.serial())
            .with("board", level.board().name())
            .with("chip", chip)
    }

    fn spawn_sticker(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        owner: Entity,
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
                Localized::from(Self::sticker_text(level)),
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

#[cfg(test)]
mod tests {
    use ld60_locale::Locale;

    use super::*;
    use crate::workshop::levels::Levels;

    #[test]
    fn stickers_format() {
        let locale = Locale::on_disk();
        for source in Levels::on_disk().levels() {
            let level = source.load().unwrap();
            locale.assert_formats([Handheld::sticker_text(&level)]);
        }
    }
}
