mod router;

use bevy::{
    asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path},
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat},
    shader::ShaderRef,
};

pub use self::router::{Pad, Router};
use super::dot_matrix::GLYPHS;

/// Mask texels per meter of board.
const DENSITY: f32 = 6000.0;
/// How far the height of the copper spreads out under the mask, in texels.
const HEIGHT_BLUR: i32 = 2;

pub struct PcbPlugin;

impl Plugin for PcbPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "pcb.wesl");
        app.add_plugins(MaterialPlugin::<PcbMaterial>::default());
    }
}

pub type PcbMaterial = ExtendedMaterial<StandardMaterial, Pcb>;

/// A printed circuit board, drawn from a mask texture: copper in red, the
/// openings in the solder mask in green, the silkscreen in blue and the
/// height of the copper in alpha.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct Pcb {
    #[texture(100)]
    #[sampler(101)]
    mask: Handle<Image>,
}

/// The mask of a [`Pcb`] while it is drawn. Positions are in meters from the
/// middle of the board, with +Y up on the texture.
#[derive(Debug)]
pub struct PcbArt {
    size: Vec2,
    texels: UVec2,
    /// Copper, opening and silk per texel, from 0 to 1.
    layers: Vec<[f32; 3]>,
}

#[derive(Clone, Copy, Debug)]
enum Layer {
    Copper = 0,
    Opening = 1,
    Silk = 2,
}

impl Pcb {
    pub fn material(mask: Handle<Image>) -> PcbMaterial {
        ExtendedMaterial {
            base: StandardMaterial {
                // Turns the clearcoat on. The shader sets it per texel.
                clearcoat: 1.0,
                ..default()
            },
            extension: Self { mask },
        }
    }

    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("pcb.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl MaterialExtension for Pcb {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        Self::shader()
    }
}

impl PcbArt {
    /// An empty board of `size` meters, covered in solder mask.
    pub fn new(size: Vec2) -> Self {
        let texels = (size * DENSITY).ceil().as_uvec2().max(UVec2::ONE);
        Self {
            size,
            texels,
            layers: vec![[0.0; 3]; (texels.x * texels.y) as usize],
        }
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    /// A copper trace under the mask along `path`.
    pub fn trace(&mut self, path: &[Vec2], width: f32) {
        self.stroke(path, width, &[Layer::Copper]);
    }

    /// A bare copper pad of `size`, open in the mask.
    pub fn pad(&mut self, center: Vec2, size: Vec2) {
        self.fill(center, size, &[Layer::Copper, Layer::Opening]);
    }

    /// A plated hole: a bare copper ring of `radius` around a hole of
    /// `hole` radius, through which the board shows.
    pub fn via(&mut self, center: Vec2, radius: f32, hole: f32) {
        self.stroke(&[center], 2.0 * radius, &[Layer::Copper, Layer::Opening]);
        let texel_size = 1.0 / DENSITY;
        self.each(
            center,
            Vec2::splat(2.0 * hole + texel_size),
            |texel, point| {
                let inside = ((hole - point.distance(center)) / texel_size + 0.5).clamp(0.0, 1.0);
                texel[Layer::Copper as usize] *= 1.0 - inside;
            },
        );
    }

    /// A printed line along `path`.
    pub fn silk_line(&mut self, path: &[Vec2], width: f32) {
        self.stroke(path, width, &[Layer::Silk]);
    }

    /// How much room `text` in the 5x7 font takes, `height` tall.
    pub fn text_size(text: &str, height: f32) -> Vec2 {
        let dot = height / 7.0;
        let columns = (text.chars().count() * 6).saturating_sub(1);
        Vec2::new(columns as f32 * dot, height)
    }

    /// Printed `text` in the 5x7 font, `height` tall, with its top left at
    /// `at`.
    pub fn silk_text(&mut self, text: &str, at: Vec2, height: f32) {
        let dot = height / 7.0;
        for (index, character) in text.chars().enumerate() {
            let Some(glyph) = (character as usize)
                .checked_sub(' ' as usize)
                .and_then(|index| GLYPHS.get(index))
            else {
                continue;
            };
            let left = at.x + index as f32 * 6.0 * dot;
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..5 {
                    if bits & (0x10 >> column) != 0 {
                        let center = Vec2::new(
                            left + (column as f32 + 0.5) * dot,
                            at.y - (row as f32 + 0.5) * dot,
                        );
                        self.fill(center, Vec2::splat(dot), &[Layer::Silk]);
                    }
                }
            }
        }
    }

    /// The mask texture, with the copper's height smoothed by the mask on
    /// top.
    pub fn image(self) -> Image {
        let (width, height) = (self.texels.x as i32, self.texels.y as i32);
        let copper = |x: i32, y: i32| {
            let (x, y) = (x.clamp(0, width - 1), y.clamp(0, height - 1));
            self.layers[(y * width + x) as usize][Layer::Copper as usize]
        };
        let area = ((2 * HEIGHT_BLUR + 1) * (2 * HEIGHT_BLUR + 1)) as f32;
        let mut data = Vec::with_capacity(self.layers.len() * 4);
        for y in 0..height {
            for x in 0..width {
                let texel = self.layers[(y * width + x) as usize];
                let mut sum = 0.0;
                for dy in -HEIGHT_BLUR..=HEIGHT_BLUR {
                    for dx in -HEIGHT_BLUR..=HEIGHT_BLUR {
                        sum += copper(x + dx, y + dy);
                    }
                }
                let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
                // Print never goes onto bare copper or into holes.
                let silk = texel[2] * (1.0 - texel[1]);
                data.extend([byte(texel[0]), byte(texel[1]), byte(silk), byte(sum / area)]);
            }
        }
        let mut image = Image::new(
            Extent3d {
                width: self.texels.x,
                height: self.texels.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::ClampToEdge,
            address_mode_v: ImageAddressMode::ClampToEdge,
            ..ImageSamplerDescriptor::linear()
        });
        image
    }

    /// Covers `layers` along `path`, `width` wide with round ends.
    fn stroke(&mut self, path: &[Vec2], width: f32, layers: &[Layer]) {
        let radius = width / 2.0;
        let pairs: Vec<_> = match path {
            [only] => vec![(*only, *only)],
            _ => path.windows(2).map(|pair| (pair[0], pair[1])).collect(),
        };
        for (a, b) in pairs {
            let center = (a + b) / 2.0;
            let extent = (a - b).abs() + Vec2::splat(width + 2.0 / DENSITY);
            let texel_size = 1.0 / DENSITY;
            self.each(center, extent, |texel, point| {
                let along = b - a;
                let t = if along == Vec2::ZERO {
                    0.0
                } else {
                    ((point - a).dot(along) / along.length_squared()).clamp(0.0, 1.0)
                };
                let distance = point.distance(a + along * t);
                let cover = ((radius - distance) / texel_size + 0.5).clamp(0.0, 1.0);
                for &layer in layers {
                    texel[layer as usize] = texel[layer as usize].max(cover);
                }
            });
        }
    }

    /// Covers `layers` in the rectangle at `center` of `size`.
    fn fill(&mut self, center: Vec2, size: Vec2, layers: &[Layer]) {
        self.paint(center, size, |texel, inside| {
            for &layer in layers {
                texel[layer as usize] = texel[layer as usize].max(inside);
            }
        });
    }

    /// Calls `apply` with how much of each texel the rectangle at `center` of
    /// `size` covers.
    fn paint(&mut self, center: Vec2, size: Vec2, mut apply: impl FnMut(&mut [f32; 3], f32)) {
        let texel_size = 1.0 / DENSITY;
        self.each(center, size + Vec2::splat(texel_size), |texel, point| {
            let off = (point - center).abs() - size / 2.0;
            let inside = (0.5 - off.max_element() / texel_size).clamp(0.0, 1.0);
            if inside > 0.0 {
                apply(texel, inside);
            }
        });
    }

    /// Calls `visit` with each texel in the rectangle at `center` of `size`
    /// and the point at its middle.
    fn each(&mut self, center: Vec2, size: Vec2, mut visit: impl FnMut(&mut [f32; 3], Vec2)) {
        let to_texel = |point: Vec2| {
            Vec2::new(
                (point.x / self.size.x + 0.5) * self.texels.x as f32,
                (0.5 - point.y / self.size.y) * self.texels.y as f32,
            )
        };
        let a = to_texel(center - size / 2.0);
        let b = to_texel(center + size / 2.0);
        let low = a.min(b).floor().max(Vec2::ZERO).as_uvec2();
        let high = a.max(b).ceil().as_uvec2().min(self.texels);
        for y in low.y..high.y {
            for x in low.x..high.x {
                let point = Vec2::new(
                    ((x as f32 + 0.5) / self.texels.x as f32 - 0.5) * self.size.x,
                    (0.5 - (y as f32 + 0.5) / self.texels.y as f32) * self.size.y,
                );
                let index = (y * self.texels.x + x) as usize;
                visit(&mut self.layers[index], point);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texel(art: &PcbArt, point: Vec2) -> [f32; 3] {
        let x = ((point.x / art.size.x + 0.5) * art.texels.x as f32) as u32;
        let y = ((0.5 - point.y / art.size.y) * art.texels.y as f32) as u32;
        art.layers[(y * art.texels.x + x) as usize]
    }

    #[test]
    fn pads_are_open_copper_and_traces_stay_under_the_mask() {
        let mut art = PcbArt::new(Vec2::new(0.02, 0.01));
        art.pad(Vec2::new(-0.005, 0.0), Vec2::splat(0.002));
        art.trace(&[Vec2::new(-0.005, 0.0), Vec2::new(0.005, 0.0)], 0.0005);
        art.silk_text("U1", Vec2::new(0.002, 0.004), 0.002);
        assert_eq!(texel(&art, Vec2::new(-0.005, 0.0))[..2], [1.0, 1.0]);
        assert_eq!(texel(&art, Vec2::new(0.002, 0.0))[..2], [1.0, 0.0]);
        assert_eq!(texel(&art, Vec2::new(0.002, 0.002))[0], 0.0);
        assert!(art.layers.iter().any(|texel| texel[2] > 0.0));
    }
}
