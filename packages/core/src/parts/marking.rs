use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::dot_matrix::GLYPHS;

/// Pixels per dot of the font.
const DOT: usize = 3;
/// Dots of space around the text and between its lines.
const MARGIN: usize = 2;
const LEADING: usize = 2;
/// More room at the start of the lines, for the dimple by pin 1.
const DIMPLE_ROOM: usize = 4;

/// Lines of text etched into the black top of a chip, in the 5x7 font.
pub struct Marking;

impl Marking {
    /// An image of `lines` in `ink` on `body`, centered after the room for
    /// the dimple. The lines run down the image, from its left edge to its
    /// right, for a chip whose top is long along V.
    pub fn image(lines: &[&str], ink: Color, body: Color) -> Image {
        let columns = lines
            .iter()
            .map(|line| (line.chars().count() * 6).saturating_sub(1))
            .max()
            .unwrap_or(0);
        let rows = lines.len() * (7 + LEADING) - LEADING;
        let (wide, tall) = (columns + 2 * MARGIN + DIMPLE_ROOM, rows + 2 * MARGIN);
        let mut dots = vec![false; wide * tall];
        for (line_index, line) in lines.iter().enumerate() {
            let left =
                MARGIN + DIMPLE_ROOM + (columns - (line.chars().count() * 6).saturating_sub(1)) / 2;
            let top = MARGIN + line_index * (7 + LEADING);
            for (index, character) in line.chars().enumerate() {
                let Some(glyph) = (character as usize)
                    .checked_sub(' ' as usize)
                    .and_then(|index| GLYPHS.get(index))
                else {
                    continue;
                };
                for (row, bits) in glyph.iter().enumerate() {
                    for column in 0..5 {
                        if bits & (0x10 >> column) != 0 {
                            dots[(top + row) * wide + left + index * 6 + column] = true;
                        }
                    }
                }
            }
        }
        // The text runs along V: its rows go across U.
        let (width, height) = (tall * DOT, wide * DOT);
        let [ink, body] = [ink, body].map(|color| color.to_srgba().to_u8_array());
        let mut data = Vec::with_capacity(width * height * 4);
        for y in 0..height {
            for x in 0..width {
                let (column, row) = (y / DOT, tall - 1 - x / DOT);
                data.extend(if dots[row * wide + column] { ink } else { body });
            }
        }
        let mut image = Image::new(
            Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::linear();
        image
    }
}
