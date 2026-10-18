use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use rand::{RngExt, SeedableRng, rngs::SmallRng};

/// Voxels along each side of the noise, which repeats seamlessly.
const SIZE: usize = 64;
const SEED: u64 = 61;
/// Cells along each side for the octaves of the cloud shapes.
const SHAPE_CELLS: [usize; 3] = [4, 8, 16];
/// And for the finer noise that wears away their edges.
const DETAIL_CELLS: [usize; 3] = [8, 16, 32];
const OCTAVE_WEIGHTS: [f32; 3] = [0.625, 0.25, 0.125];

/// One random point in each cell of a repeating grid.
struct Points {
    cells: usize,
    points: Vec<Vec3>,
}

impl Points {
    fn new(cells: usize, rng: &mut SmallRng) -> Self {
        let points = (0..cells * cells * cells)
            .map(|_| Vec3::new(rng.random(), rng.random(), rng.random()))
            .collect();
        Self { cells, points }
    }

    /// 1 on a point, falling to 0 a cell away from the nearest one.
    fn cellular(&self, p: Vec3) -> f32 {
        let n = self.cells as i64;
        let q = p * self.cells as f32;
        let cell = q.floor();
        let mut nearest = f32::MAX;
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let c = cell + Vec3::new(dx as f32, dy as f32, dz as f32);
                    let wrap = |v: f32| (v as i64).rem_euclid(n) as usize;
                    let index = wrap(c.x) + self.cells * (wrap(c.y) + self.cells * wrap(c.z));
                    nearest = nearest.min((c + self.points[index] - q).length_squared());
                }
            }
        }
        1.0 - nearest.sqrt().min(1.0)
    }
}

/// Random values on the corners of a repeating grid.
struct Lattice {
    cells: usize,
    values: Vec<f32>,
}

impl Lattice {
    fn new(cells: usize, rng: &mut SmallRng) -> Self {
        let values = (0..cells * cells * cells).map(|_| rng.random()).collect();
        Self { cells, values }
    }

    /// Smoothly blended values from 0 to 1.
    fn smooth(&self, p: Vec3) -> f32 {
        let n = self.cells as i64;
        let q = p * self.cells as f32;
        let cell = q.floor();
        let f = q - cell;
        let f = f * f * (3.0 - 2.0 * f);
        let at = |dx: f32, dy: f32, dz: f32| {
            let wrap = |v: f32| (v as i64).rem_euclid(n) as usize;
            let c = cell + Vec3::new(dx, dy, dz);
            self.values[wrap(c.x) + self.cells * (wrap(c.y) + self.cells * wrap(c.z))]
        };
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let y0 = lerp(
            lerp(at(0.0, 0.0, 0.0), at(1.0, 0.0, 0.0), f.x),
            lerp(at(0.0, 1.0, 0.0), at(1.0, 1.0, 0.0), f.x),
            f.y,
        );
        let y1 = lerp(
            lerp(at(0.0, 0.0, 1.0), at(1.0, 0.0, 1.0), f.x),
            lerp(at(0.0, 1.0, 1.0), at(1.0, 1.0, 1.0), f.x),
            f.y,
        );
        lerp(y0, y1, f.z)
    }
}

/// Noise the clouds are made of, after Schneider, "The Real-time Volumetric
/// Cloudscapes of Horizon Zero Dawn".
///
/// Red holds billowy shapes: smooth noise pushed up where cellular noise is
/// high, so it bulges in round lumps. Green holds finer cellular noise that
/// wears away their edges into wisps.
pub(super) fn noise() -> Image {
    let mut rng = SmallRng::seed_from_u64(SEED);
    let points: Vec<Points> = SHAPE_CELLS
        .iter()
        .chain(&DETAIL_CELLS)
        .map(|&cells| Points::new(cells, &mut rng))
        .collect();
    let lattices: Vec<Lattice> = SHAPE_CELLS
        .iter()
        .map(|&cells| Lattice::new(cells, &mut rng))
        .collect();
    let mut shapes = Vec::with_capacity(SIZE * SIZE * SIZE);
    let mut details = Vec::with_capacity(SIZE * SIZE * SIZE);
    for z in 0..SIZE {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let p = (Vec3::new(x as f32, y as f32, z as f32) + 0.5) / SIZE as f32;
                let fbm = |values: &mut dyn Iterator<Item = f32>| {
                    values.zip(OCTAVE_WEIGHTS).map(|(v, w)| v * w).sum::<f32>()
                };
                let cells = fbm(&mut points[..3].iter().map(|points| points.cellular(p)));
                let smooth = fbm(&mut lattices.iter().map(|lattice| lattice.smooth(p)));
                let detail = fbm(&mut points[3..].iter().map(|points| points.cellular(p)));
                shapes.push(cells + smooth * (1.0 - cells));
                details.push(detail);
            }
        }
    }
    // Both spread over the whole range, so coverage means the same for
    // any seed.
    let stretch = |values: &[f32]| {
        let (low, high) = values
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        let span = (high - low).max(f32::EPSILON);
        values
            .iter()
            .map(|&v| ((v - low) / span * 255.0) as u8)
            .collect::<Vec<_>>()
    };
    let data = stretch(&shapes)
        .into_iter()
        .zip(stretch(&details))
        .flat_map(|(shape, detail)| [shape, detail])
        .collect();
    let mut image = Image::new(
        Extent3d {
            width: SIZE as u32,
            height: SIZE as u32,
            depth_or_array_layers: SIZE as u32,
        },
        TextureDimension::D3,
        data,
        TextureFormat::Rg8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}
