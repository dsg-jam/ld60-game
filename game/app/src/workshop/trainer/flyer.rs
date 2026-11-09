use std::f32::consts::TAU;

use bevy::prelude::*;
use ld60_devices::MARKS;
use ld60_game_parts::drawing::Drawing;
use ld60_game_parts::paper::PAPER;
use ld60_game_parts::script::Word;
use rand::SeedableRng;
use rand::rngs::SmallRng;

const SIZE: Vec2 = Vec2::new(0.148, 0.105);
/// A red, cheaply printed.
const INK: Color = Color::srgb_u8(0x9a, 0x2a, 0x24);
/// The dials of the strip: how far apart, how big, and their lines.
const DIAL_SPACING: f32 = 0.048;
const DIAL_RADIUS: f32 = 0.017;
const LINE: f32 = 0.0011;
const MARK: f32 = 0.0028;
/// Grid steps of the script, for the title and the small print.
const TITLE_STEP: f32 = 0.0034;
const SMALL_STEP: f32 = 0.0016;

/// A customer's flyer for what a watch should do, in their own script.
/// A strip of three dials shows the hand going on one mark each tick.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Flyer {
    pub(crate) seed: u64,
}

impl Flyer {
    pub(super) fn on_add(
        add: On<Add<Flyer>>,
        flyers: Query<&Flyer>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok(flyer) = flyers.get(add.entity) else {
            return;
        };
        let image = images.add(Self::draw(flyer.seed));
        super::sheet::lay(
            &mut commands,
            add.entity,
            (image, SIZE, ("flyer", flyer.seed)),
            "click-flyer",
            &mut meshes,
            &mut materials,
        );
    }

    fn draw(seed: u64) -> Image {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut drawing = Drawing::new(SIZE, PAPER, INK);
        let title = Word::random(&mut rng, 3..5);
        drawing.word(&title, Vec2::new(0.0, -0.036), TITLE_STEP, 0.0014);
        // The word for a tick, the same between each pair of dials.
        let tick = Word::random(&mut rng, 1..2);
        for index in 0..3u8 {
            let center = Vec2::new(f32::from(index) - 1.0, 0.0) * DIAL_SPACING;
            Self::dial(&mut drawing, center, index);
            if index < 2 {
                let between = center + Vec2::X * DIAL_SPACING / 2.0;
                Self::arrow(&mut drawing, between);
                drawing.word(&tick, between - Vec2::Y * 0.009, SMALL_STEP, 0.0008);
            }
        }
        for row in 0..2 {
            let words = Word::random(&mut rng, 4..9);
            let y = 0.03 + row as f32 * 0.008;
            drawing.word(&words, Vec2::new(0.0, y), SMALL_STEP, 0.0008);
        }
        drawing.image()
    }

    /// A dial with its marks and the hand on `mark`. Mark 0 is at the top.
    fn dial(drawing: &mut Drawing, center: Vec2, mark: u8) {
        drawing.ring(center, DIAL_RADIUS, LINE);
        let point = |mark: f32, radius: f32| {
            let angle = TAU * mark / f32::from(MARKS);
            center + Vec2::new(angle.sin(), -angle.cos()) * radius
        };
        for index in 0..MARKS {
            let inner = DIAL_RADIUS - MARK * if index == 0 { 1.8 } else { 1.0 };
            let index = f32::from(index);
            drawing.stroke(
                point(index, inner),
                point(index, DIAL_RADIUS - LINE),
                LINE * 0.7,
            );
        }
        drawing.stroke(
            center,
            point(f32::from(mark), DIAL_RADIUS * 0.75),
            LINE * 1.4,
        );
        drawing.ring(center, LINE, LINE);
    }

    fn arrow(drawing: &mut Drawing, at: Vec2) {
        let half = 0.0035;
        let tip = at + Vec2::X * half;
        drawing.stroke(at - Vec2::X * half, tip, LINE);
        drawing.stroke(tip, tip + Vec2::new(-0.002, -0.002), LINE);
        drawing.stroke(tip, tip + Vec2::new(-0.002, 0.002), LINE);
    }
}
