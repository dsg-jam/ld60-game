use bevy::prelude::*;
use ld60_devices::{Annunciator, Board};

use super::screen::UiCanvas;

/// Canvas pixels around the digits.
const MARGIN: f32 = 20.0;
/// Width of the lamp column left of the digits.
const LAMP_COLUMN: f32 = 56.0;
const DIGIT_SIZE: Vec2 = Vec2::new(60.0, 104.0);
const DIGIT_GAP: f32 = 20.0;
const SEGMENT_THICKNESS: f32 = 10.0;
/// The error lamp is a small "E" made of segments.
const LAMP_SIZE: Vec2 = Vec2::new(24.0, 40.0);
const LAMP_THICKNESS: f32 = 5.0;
/// Segments a, d, e, f and g.
const LETTER_E: u8 = 0x79;
/// Width of the column right of the digits that prints the lamp words.
const WORD_COLUMN: f32 = 150.0;
const WORD_FONT_SIZE: f32 = 40.0;
const WORDS: [Annunciator; 2] = [Annunciator::Open, Annunciator::Shut];

const BACKGROUND: Color = Color::srgb_u8(0x9c, 0xab, 0x8a);
const INK: Color = Color::srgb_u8(0x23, 0x2b, 0x22);
/// Unlit segments still show faintly, like on a real LCD.
const GHOST: Color = Color::srgba_u8(0x23, 0x2b, 0x22, 0x14);

/// Segments a to g of the digits 0 to 9, bit 0 is a.
const DIGITS: [u8; 10] = [0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F];

pub struct LcdPlugin;

impl Plugin for LcdPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show);
    }
}

/// What an LCD shows. Digit values above 9 light a scrambled pattern.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct LcdDisplay {
    pub digits: Vec<u8>,
    pub minus: bool,
    pub error: bool,
    pub open: bool,
    pub shut: bool,
}

#[derive(Component, Debug)]
struct Segment {
    source: Entity,
    digit: usize,
    bit: u8,
}

/// A lamp drawn with segments.
#[derive(Component, Debug)]
struct Lamp {
    source: Entity,
    kind: Annunciator,
}

/// A lamp printed as a word.
#[derive(Component, Debug)]
struct Word {
    source: Entity,
    kind: Annunciator,
}

impl LcdDisplay {
    fn is_lit(&self, annunciator: Annunciator) -> bool {
        match annunciator {
            Annunciator::Minus => self.minus,
            Annunciator::Error => self.error,
            Annunciator::Open => self.open,
            Annunciator::Shut => self.shut,
        }
    }
}

/// Pixel size of the canvas for `digits` digits, with the lamp words if
/// `words` is set.
pub fn canvas_size(digits: usize, words: bool) -> UVec2 {
    let words = if words { DIGIT_GAP + WORD_COLUMN } else { 0.0 };
    let width =
        2.0 * MARGIN + LAMP_COLUMN + digits as f32 * (DIGIT_SIZE.x + DIGIT_GAP) - DIGIT_GAP + words;
    let height = 2.0 * MARGIN + DIGIT_SIZE.y;
    Vec2::new(width, height).as_uvec2()
}

/// Spawns an LCD showing the [`LcdDisplay`] of `source` and returns its
/// texture. It draws only the lamps that `board` wires. With `words`, the
/// OPEN and SHUT lamps follow the digits.
pub fn spawn(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    source: Entity,
    digits: usize,
    words: bool,
    board: &Board,
) -> Handle<Image> {
    let canvas = UiCanvas::spawn(commands, images, canvas_size(digits, words), source);
    let digit_segments = segment_rects(DIGIT_SIZE, SEGMENT_THICKNESS);
    let lamp_segments = segment_rects(LAMP_SIZE, LAMP_THICKNESS);
    let lamp_origin = Vec2::splat(MARGIN);
    let minus = digit_segments[6];
    let wired = |kind: Annunciator| board.ann_bit(kind).is_some();

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(BACKGROUND),
            canvas.target(),
        ))
        .with_children(|root| {
            if wired(Annunciator::Minus) {
                root.spawn((
                    rect(
                        lamp_origin + minus.0.with_x(0.0),
                        minus.1.with_x(LAMP_SIZE.x),
                    ),
                    BackgroundColor(GHOST),
                    Lamp {
                        source,
                        kind: Annunciator::Minus,
                    },
                ));
            }
            for (bit, (position, size)) in lamp_segments.iter().enumerate() {
                if wired(Annunciator::Error) && LETTER_E & (1 << bit) != 0 {
                    root.spawn((
                        rect(lamp_origin + *position, *size),
                        BackgroundColor(GHOST),
                        Lamp {
                            source,
                            kind: Annunciator::Error,
                        },
                    ));
                }
            }
            for digit in 0..digits {
                let origin = Vec2::new(
                    MARGIN + LAMP_COLUMN + digit as f32 * (DIGIT_SIZE.x + DIGIT_GAP),
                    MARGIN,
                );
                for (bit, (position, size)) in digit_segments.iter().enumerate() {
                    root.spawn((
                        rect(origin + *position, *size),
                        BackgroundColor(GHOST),
                        Segment {
                            source,
                            digit,
                            bit: bit as u8,
                        },
                    ));
                }
            }
            if words {
                let left = MARGIN + LAMP_COLUMN + digits as f32 * (DIGIT_SIZE.x + DIGIT_GAP);
                let size = Vec2::new(WORD_COLUMN, DIGIT_SIZE.y / WORDS.len() as f32);
                for (row, kind) in WORDS
                    .into_iter()
                    .enumerate()
                    .filter(|(_, kind)| wired(*kind))
                {
                    root.spawn((
                        rect(Vec2::new(left, MARGIN + row as f32 * size.y), size),
                        Text::new(kind.mark()),
                        TextFont {
                            font_size: FontSize::Px(WORD_FONT_SIZE),
                            ..default()
                        },
                        TextColor(GHOST),
                        Word { source, kind },
                    ));
                }
            }
        });
    canvas.image
}

/// Position and size of segments a to g within a digit of `size`.
fn segment_rects(size: Vec2, thickness: f32) -> [(Vec2, Vec2); 7] {
    let t = thickness;
    let half = size.y / 2.0;
    let side = half - 1.5 * t;
    let bar = size.x - 2.0 * t;
    [
        (Vec2::new(t, 0.0), Vec2::new(bar, t)),
        (Vec2::new(size.x - t, t), Vec2::new(t, side)),
        (Vec2::new(size.x - t, half + t / 2.0), Vec2::new(t, side)),
        (Vec2::new(t, size.y - t), Vec2::new(bar, t)),
        (Vec2::new(0.0, half + t / 2.0), Vec2::new(t, side)),
        (Vec2::new(0.0, t), Vec2::new(t, side)),
        (Vec2::new(t, half - t / 2.0), Vec2::new(bar, t)),
    ]
}

fn rect(position: Vec2, size: Vec2) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(position.x),
        top: px(position.y),
        width: px(size.x),
        height: px(size.y),
        ..default()
    }
}

/// Lit segments of a digit cell.
///
/// Values above 9 light a scrambled pattern, like a display driver fed garbage.
fn pattern(value: u8) -> u8 {
    match DIGITS.get(usize::from(value)) {
        Some(&segments) => segments,
        None => value.wrapping_mul(37).wrapping_add(11) & 0x7F | 0x08,
    }
}

fn show(
    displays: Query<&LcdDisplay, Changed<LcdDisplay>>,
    mut segments: Query<(&Segment, &mut BackgroundColor)>,
    mut lamps: Query<(&Lamp, &mut BackgroundColor), Without<Segment>>,
    mut words: Query<(&Word, &mut TextColor)>,
) {
    let ink = |lit: bool| if lit { INK } else { GHOST };
    for (segment, mut color) in &mut segments {
        if let Ok(display) = displays.get(segment.source) {
            let value = display.digits.get(segment.digit).copied().unwrap_or(0);
            color.0 = ink(pattern(value) & (1 << segment.bit) != 0);
        }
    }
    for (lamp, mut color) in &mut lamps {
        if let Ok(display) = displays.get(lamp.source) {
            color.0 = ink(display.is_lit(lamp.kind));
        }
    }
    for (word, mut color) in &mut words {
        if let Ok(display) = displays.get(word.source) {
            color.0 = ink(display.is_lit(word.kind));
        }
    }
}
