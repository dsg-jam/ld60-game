use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::dot_matrix::GLYPHS;

/// Pixels per meter of the plate.
const RESOLUTION: f32 = 6000.0;

/// A printed front plate: labels in the 5x7 font and lines in `ink` on
/// `body`. Places are in meters from the middle, +Y towards the front.
pub struct Faceplate {
    size: UVec2,
    ink: [u8; 4],
    pixels: Vec<[u8; 4]>,
}

impl Faceplate {
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

    /// A straight line `width` meters wide.
    pub fn line(&mut self, from: Vec2, to: Vec2, width: f32) {
        let half = Vec2::splat(width / 2.0);
        self.fill(from.min(to) - half, from.max(to) + half);
    }

    /// Fills the box between two corners with ink.
    pub fn fill(&mut self, min: Vec2, max: Vec2) {
        let middle = self.size.as_vec2() / 2.0;
        let [min, max] = [min, max].map(|at| {
            (at * RESOLUTION + middle)
                .round()
                .clamp(Vec2::ZERO, self.size.as_vec2())
                .as_uvec2()
        });
        for y in min.y..max.y {
            for x in min.x..max.x {
                self.pixels[(y * self.size.x + x) as usize] = self.ink;
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
