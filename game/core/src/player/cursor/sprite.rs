use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::{CursorIcon, CustomCursor, CustomCursorImage},
};

/// Cursor pixels with straight alpha, and the click point inside them.
#[derive(Clone, Debug)]
pub(super) struct Sprite {
    size: IVec2,
    hotspot: IVec2,
    pixels: Vec<LinearRgba>,
}

impl Sprite {
    /// Pixel art where `#` is `ink`, `.` is `fill` and a space is transparent.
    ///
    /// `hotspot` is in art pixels.
    pub(super) fn pixel_art(
        rows: &[&str],
        scale: i32,
        hotspot: IVec2,
        fill: Color,
        ink: Color,
    ) -> Self {
        let width = rows.iter().map(|row| row.len()).max().unwrap_or(0) as i32;
        let size = IVec2::new(width, rows.len() as i32) * scale;
        Self::new(size, hotspot * scale + scale / 2, |pixel| {
            let art = pixel / scale;
            match rows[art.y as usize].as_bytes().get(art.x as usize) {
                Some(b'#') => ink.to_linear(),
                Some(b'.') => fill.to_linear(),
                _ => LinearRgba::NONE,
            }
        })
    }

    /// Filled circle with an outline and soft edges, clicking in its center.
    pub(super) fn dot(diameter: f32, outline: f32, fill: Color, ink: Color) -> Self {
        let pixels = diameter.ceil() as i32;
        let radius = diameter / 2.0;
        let (fill, ink) = (fill.to_linear(), ink.to_linear());
        Self::new(IVec2::splat(pixels), IVec2::splat(pixels / 2), |pixel| {
            let distance = (pixel.as_vec2() + 0.5 - radius).length();
            let coverage = (radius - distance + 0.5).clamp(0.0, 1.0);
            let inner = (radius - outline - distance + 0.5).clamp(0.0, 1.0);
            let color = ink.mix(&fill, inner);
            color.with_alpha(color.alpha * coverage)
        })
    }

    /// Crossfade from `self` at 0 to `other` at 1, with the hotspots on top
    /// of each other.
    pub(super) fn blend(&self, other: &Self, t: f32) -> Self {
        let hotspot = self.hotspot.max(other.hotspot);
        let size = hotspot + (self.size - self.hotspot).max(other.size - other.hotspot);
        Self::new(size, hotspot, |pixel| {
            let a = self.sample(pixel - hotspot + self.hotspot);
            let b = other.sample(pixel - hotspot + other.hotspot);
            let (wa, wb) = (a.alpha * (1.0 - t), b.alpha * t);
            let alpha = wa + wb;
            if alpha == 0.0 {
                return LinearRgba::NONE;
            }
            let color = (a * wa + b * wb) * (1.0 / alpha);
            color.with_alpha(alpha)
        })
    }

    pub(super) fn icon(&self, images: &mut Assets<Image>) -> CursorIcon {
        let data = self
            .pixels
            .iter()
            .flat_map(|&pixel| Color::from(pixel).to_srgba().to_u8_array())
            .collect();
        let image = Image::new(
            Extent3d {
                width: self.size.x as u32,
                height: self.size.y as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        CursorIcon::from(CustomCursor::Image(CustomCursorImage {
            handle: images.add(image),
            hotspot: (self.hotspot.x as u16, self.hotspot.y as u16),
            ..default()
        }))
    }

    fn new(size: IVec2, hotspot: IVec2, pixel: impl Fn(IVec2) -> LinearRgba) -> Self {
        let pixels = (0..size.y)
            .flat_map(|y| (0..size.x).map(move |x| IVec2::new(x, y)))
            .map(pixel)
            .collect();
        Self {
            size,
            hotspot,
            pixels,
        }
    }

    fn sample(&self, pixel: IVec2) -> LinearRgba {
        if pixel.cmplt(IVec2::ZERO).any() || pixel.cmpge(self.size).any() {
            return LinearRgba::NONE;
        }
        self.pixels[(pixel.y * self.size.x + pixel.x) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_keeps_hotspots_aligned() {
        let dot = Sprite::dot(4.0, 1.0, Color::WHITE, Color::BLACK);
        let arrow = Sprite::pixel_art(&["#."], 1, IVec2::ZERO, Color::WHITE, Color::BLACK);
        let start = arrow.blend(&dot, 0.0);
        let end = arrow.blend(&dot, 1.0);
        assert_eq!(start.hotspot, IVec2::splat(2));
        for offset in [IVec2::ZERO, IVec2::X, IVec2::new(1, 1), IVec2::new(-2, -2)] {
            assert_eq!(
                start.sample(start.hotspot + offset),
                arrow.sample(arrow.hotspot + offset)
            );
            assert_eq!(
                end.sample(end.hotspot + offset),
                dot.sample(dot.hotspot + offset)
            );
        }
    }
}
