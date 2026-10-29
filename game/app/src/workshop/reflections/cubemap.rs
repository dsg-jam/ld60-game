use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
    },
};

/// A soft spot of light in some direction, like a lamp or a lit wall.
#[derive(Clone, Copy, Debug)]
pub(super) struct Lobe {
    pub(super) toward: Vec3,
    /// How tightly the light gathers around `toward`. About 1 over the
    /// square of its angular radius in radians.
    pub(super) sharpness: f32,
    /// Linear radiance at its middle.
    pub(super) color: Vec3,
}

impl Lobe {
    pub(super) const fn new(toward: Vec3, sharpness: f32, color: Vec3) -> Self {
        Self {
            toward,
            sharpness,
            color,
        }
    }

    fn radiance(&self, direction: Vec3) -> Vec3 {
        self.color * (self.sharpness * (direction.dot(self.toward.normalize()) - 1.0)).exp()
    }

    /// The lobe as a surface of `roughness` reflects it: wider and dimmer,
    /// with the same light in total.
    fn blurred(self, roughness: f32) -> Self {
        if roughness <= 0.0 {
            return self;
        }
        // A GGX highlight of this roughness spreads about this much around
        // the mirror direction.
        let alpha = roughness * roughness;
        let spread = 0.5 / (alpha * alpha);
        let sharpness = self.sharpness * spread / (self.sharpness + spread);
        let total = |sharpness: f32| (1.0 - (-2.0 * sharpness).exp()) / sharpness;
        Self {
            sharpness,
            color: self.color * total(self.sharpness) / total(sharpness),
            ..self
        }
    }
}

/// A cubemap of the `lobes` for the specular map of an
/// [`EnvironmentMapLight`], each mip blurred for the roughness Bevy reads it
/// at. Faces are `size` texels wide, a power of two.
pub(super) fn specular(lobes: &[Lobe], size: u32) -> Image {
    let levels = size.ilog2() + 1;
    let mut data = Vec::new();
    for face in 0..6 {
        for level in 0..levels {
            let roughness = level as f32 / (levels - 1) as f32;
            let blurred: Vec<Lobe> = lobes.iter().map(|lobe| lobe.blurred(roughness)).collect();
            let texels = size >> level;
            for y in 0..texels {
                for x in 0..texels {
                    let uv = (UVec2::new(x, y).as_vec2() + 0.5) / texels as f32 * 2.0 - 1.0;
                    let direction = direction(face, uv);
                    let radiance = blurred.iter().map(|lobe| lobe.radiance(direction)).sum();
                    data.extend_from_slice(&rgb9e5(radiance).to_le_bytes());
                }
            }
        }
    }
    cube(size, levels, data)
}

/// A cubemap that adds no light, for the diffuse map.
pub(super) fn black() -> Image {
    cube(1, 1, vec![0; 4 * 6])
}

fn cube(size: u32, levels: u32, data: Vec<u8>) -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        TextureFormat::Rgb9e5Ufloat,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = levels;
    image.data = Some(data);
    image.sampler = ImageSampler::linear();
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    image
}

/// The world direction through `uv` on cube `face`, from -1 to 1 with y
/// down. Faces go +X, -X, +Y, -Y, +Z, -Z. Bevy looks cubemaps up with Z
/// flipped.
fn direction(face: u32, uv: Vec2) -> Vec3 {
    let (u, v) = (uv.x, uv.y);
    let cube = match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    };
    (cube * Vec3::new(1.0, 1.0, -1.0)).normalize()
}

/// Packs a color into the shared exponent format of
/// [`TextureFormat::Rgb9e5Ufloat`].
fn rgb9e5(color: Vec3) -> u32 {
    const MANTISSA: i32 = 9;
    const BIAS: i32 = 15;
    const LARGEST: f32 = 65408.0;
    let color = color.clamp(Vec3::ZERO, Vec3::splat(LARGEST));
    let largest = color.max_element();
    if largest <= 0.0 {
        return 0;
    }
    let mut exponent = largest.log2().floor().max(-(BIAS as f32) - 1.0) as i32 + 1 + BIAS;
    let mut scale = 2f32.powi(exponent - BIAS - MANTISSA);
    if (largest / scale + 0.5).floor() >= (1 << MANTISSA) as f32 {
        scale *= 2.0;
        exponent += 1;
    }
    let [r, g, b] = (color / scale + 0.5).floor().to_array().map(|c| c as u32);
    r | g << 9 | b << 18 | (exponent as u32) << 27
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_shared_exponents() {
        assert_eq!(rgb9e5(Vec3::ZERO), 0);
        // 1.0 is 256 times 2^(16 - 15 - 9).
        assert_eq!(rgb9e5(Vec3::ONE), 256 | 256 << 9 | 256 << 18 | 16 << 27);
    }

    #[test]
    fn faces_look_their_way() {
        assert_eq!(direction(0, Vec2::ZERO), Vec3::X);
        assert_eq!(direction(2, Vec2::ZERO), Vec3::Y);
        assert_eq!(direction(3, Vec2::ZERO), Vec3::NEG_Y);
        // Flipped, so the +Z face holds what lies towards -Z.
        assert_eq!(direction(4, Vec2::ZERO), Vec3::NEG_Z);
    }

    #[test]
    fn blurring_keeps_the_light() {
        let lobe = Lobe::new(Vec3::Y, 50.0, Vec3::ONE);
        let blurred = lobe.blurred(0.5);
        assert!(blurred.sharpness < lobe.sharpness);
        assert!(blurred.color.x < lobe.color.x);
    }
}
