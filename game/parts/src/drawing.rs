use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::dot_matrix::GLYPHS;
use super::script::Word;

/// Pixels per meter.
const RESOLUTION: f32 = 6000.0;

/// A print in one `ink` on a plain `body`, like a front plate or a flyer:
/// text in the 5x7 font, words in the alien script, strokes and rings.
/// Places are in meters from the middle, +Y down the image.
pub struct Drawing {
    size: UVec2,
    ink: [u8; 4],
    pixels: Vec<[u8; 4]>,
}

impl Drawing {
    pub fn new(size: Vec2, body: Color, ink: Color) -> Self {
        let size = (size * RESOLUTION).ceil().as_uvec2();
        let [body, ink] = [body, ink].map(|color| color.to_srgba().to_u8_array());
        Self {
            size,
            ink,
            pixels: vec![body; (size.x * size.y) as usize],
        }
    }

    /// `text` centered on `at`, with dots `dot` meters wide.
    pub fn text(&mut self, text: &str, at: Vec2, dot: f32) {
        let columns = (text.chars().count() * 6).saturating_sub(1) as f32;
        let corner = at - Vec2::new(columns, 7.0) * dot / 2.0;
        for (index, character) in text.chars().enumerate() {
            let Some(glyph) = (character as usize)
                .checked_sub(' ' as usize)
                .and_then(|index| GLYPHS.get(index))
            else {
                continue;
            };
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..5 {
                    if bits & (0x10 >> column) != 0 {
                        let min = corner + Vec2::new((index * 6 + column) as f32, row as f32) * dot;
                        self.fill(min, min + Vec2::splat(dot));
                    }
                }
            }
        }
    }

    /// A straight stroke `width` meters wide with round ends.
    pub fn stroke(&mut self, from: Vec2, to: Vec2, width: f32) {
        let half = width / 2.0;
        self.paint(from.min(to) - half, from.max(to) + half, |at| {
            let along = to - from;
            let t =
                ((at - from).dot(along) / along.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
            at.distance(from + along * t) <= half
        });
    }

    /// A circle of `radius` drawn with a line `width` meters wide.
    pub fn ring(&mut self, center: Vec2, radius: f32, width: f32) {
        let reach = Vec2::splat(radius + width);
        self.paint(center - reach, center + reach, |at| {
            (at.distance(center) - radius).abs() <= width / 2.0
        });
    }

    /// `word` in the alien script centered on `at`, with grid steps `step`
    /// meters apart and strokes `width` meters wide.
    pub fn word(&mut self, word: &Word, at: Vec2, step: f32, width: f32) {
        for (a, b) in word.strokes(false) {
            let flip = Vec2::new(1.0, -1.0);
            self.stroke(at + a * flip * step, at + b * flip * step, width);
        }
    }

    /// Fills the box between two corners with ink.
    pub fn fill(&mut self, min: Vec2, max: Vec2) {
        self.paint(min, max, |_| true);
    }

    /// Inks the pixels between two corners whose middles pass `inside`.
    fn paint(&mut self, min: Vec2, max: Vec2, inside: impl Fn(Vec2) -> bool) {
        let middle = self.size.as_vec2() / 2.0;
        let [low, high] = [min, max].map(|at| {
            (at * RESOLUTION + middle)
                .round()
                .clamp(Vec2::ZERO, self.size.as_vec2())
                .as_uvec2()
        });
        for y in low.y..high.y {
            for x in low.x..high.x {
                let at = (UVec2::new(x, y).as_vec2() + 0.5 - middle) / RESOLUTION;
                if inside(at) {
                    self.pixels[(y * self.size.x + x) as usize] = self.ink;
                }
            }
        }
    }

    pub fn image(self) -> Image {
        let mut image = Image::new(
            Extent3d {
                width: self.size.x,
                height: self.size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            self.pixels.into_flattened(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::linear();
        image
    }
}
