use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Ink coverage on a grid of pixels, drawn with round ended strokes.
pub(super) struct Canvas {
    size: UVec2,
    ink: Vec<f32>,
}

impl Canvas {
    pub(super) fn new(size: UVec2) -> Self {
        Self {
            size,
            ink: vec![0.0; (size.x * size.y) as usize],
        }
    }

    /// A stroke from `a` to `b` in pixels from the top left corner, `width`
    /// pixels wide.
    pub(super) fn stroke(&mut self, a: Vec2, b: Vec2, width: f32) {
        let radius = width / 2.0;
        let low = (a.min(b) - radius - 1.0).max(Vec2::ZERO).as_uvec2();
        let high = (a.max(b) + radius + 1.0)
            .min((self.size - 1).as_vec2())
            .as_uvec2();
        let along = b - a;
        let length = along.length_squared().max(1e-6);
        for y in low.y..=high.y {
            for x in low.x..=high.x {
                let point = Vec2::new(x as f32, y as f32) + 0.5;
                let t = ((point - a).dot(along) / length).clamp(0.0, 1.0);
                let distance = point.distance(a + along * t);
                let cover = (radius + 0.5 - distance).clamp(0.0, 1.0);
                let ink = &mut self.ink[(y * self.size.x + x) as usize];
                *ink = ink.max(cover);
            }
        }
    }

    /// White ink on nothing, with `levels` levels of detail, each half the
    /// size of the one before.
    pub(super) fn image(&self, levels: u32) -> Image {
        let mut data = Vec::new();
        let (mut size, mut ink) = (self.size, self.ink.clone());
        for level in 0..levels {
            if level > 0 {
                let half = size / 2;
                ink = (0..half.y)
                    .flat_map(|y| (0..half.x).map(move |x| (x, y)))
                    .map(|(x, y)| {
                        let at = |dx, dy| ink[((2 * y + dy) * size.x + 2 * x + dx) as usize];
                        (at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1)) / 4.0
                    })
                    .collect();
                size = half;
            }
            data.extend(
                ink.iter()
                    .flat_map(|&cover| [255, 255, 255, (cover * 255.0).round() as u8]),
            );
        }
        let mut image = Image::new_uninit(
            Extent3d {
                width: self.size.x,
                height: self.size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.data = Some(data);
        image.texture_descriptor.mip_level_count = levels;
        image.sampler = ImageSampler::linear();
        image
    }
}
