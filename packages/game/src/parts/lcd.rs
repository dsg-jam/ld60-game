use bevy::prelude::*;

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

const BACKGROUND: Color = Color::srgb_u8(0x9c, 0xab, 0x8a);
const INK: Color = Color::srgb_u8(0x23, 0x2b, 0x22);
/// Unlit segments still show faintly, like on a real LCD.
const GHOST: Color = Color::srgba_u8(0x23, 0x2b, 0x22, 0x14);

/// Segments a to g of the digits 0 to 9, bit 0 is a.
const DIGITS: [u8; 10] = [0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F];

pub(crate) struct LcdPlugin;

impl Plugin for LcdPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show);
    }
}

/// What an LCD shows. Digit values above 9 light a scrambled pattern.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LcdDisplay {
    pub(crate) digits: Vec<u8>,
    pub(crate) minus: bool,
    pub(crate) error: bool,
}

#[derive(Component, Debug)]
struct Segment {
    source: Entity,
    digit: usize,
    bit: u8,
}

#[derive(Component, Debug)]
struct Lamp {
    source: Entity,
    kind: LampKind,
}

#[derive(Clone, Copy, Debug)]
enum LampKind {
    Minus,
    Error,
}

impl LampKind {
    fn is_lit(self, display: &LcdDisplay) -> bool {
        match self {
            Self::Minus => display.minus,
            Self::Error => display.error,
        }
    }
}

/// Pixel size of the canvas for `digits` digits.
pub(crate) fn canvas_size(digits: usize) -> UVec2 {
    let width = 2.0 * MARGIN + LAMP_COLUMN + digits as f32 * (DIGIT_SIZE.x + DIGIT_GAP) - DIGIT_GAP;
    let height = 2.0 * MARGIN + DIGIT_SIZE.y;
    Vec2::new(width, height).as_uvec2()
}

/// Spawns an LCD showing the [`LcdDisplay`] of `source` and returns its
/// texture.
pub(crate) fn spawn(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    source: Entity,
    digits: usize,
) -> Handle<Image> {
    let canvas = UiCanvas::spawn(commands, images, canvas_size(digits));
    let digit_segments = segment_rects(DIGIT_SIZE, SEGMENT_THICKNESS);
    let lamp_segments = segment_rects(LAMP_SIZE, LAMP_THICKNESS);
    let lamp_origin = Vec2::splat(MARGIN);
    let minus = digit_segments[6];

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(BACKGROUND),
            UiTargetCamera(canvas.camera),
        ))
        .with_children(|root| {
            root.spawn((
                rect(
                    lamp_origin + minus.0.with_x(0.0),
                    minus.1.with_x(LAMP_SIZE.x),
                ),
                BackgroundColor(GHOST),
                Lamp {
                    source,
                    kind: LampKind::Minus,
                },
            ));
            for (bit, (position, size)) in lamp_segments.iter().enumerate() {
                if LETTER_E & (1 << bit) != 0 {
                    root.spawn((
                        rect(lamp_origin + *position, *size),
                        BackgroundColor(GHOST),
                        Lamp {
                            source,
                            kind: LampKind::Error,
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
            color.0 = ink(lamp.kind.is_lit(display));
        }
    }
}
