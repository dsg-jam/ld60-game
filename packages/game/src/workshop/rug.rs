use std::ops::Range;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use rand::{Rng, RngExt, SeedableRng, rngs::SmallRng};

use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;

/// The knotted part, long along Z.
pub(super) const SIZE: Vec3 = Vec3::new(1.3, 0.008, 1.8);
const RED: Color = Color::srgb_u8(0x7a, 0x24, 0x1e);
const WOOL: Color = Color::srgb_u8(0x9e, 0x8c, 0x6c);

const SEED: u64 = 12;
/// Space between the tassels along each end.
const SPACING: f32 = 0.008;
const LENGTH: Range<f32> = 0.035..0.07;
const WIDTH: f32 = 0.004;
const SEGMENTS: usize = 8;
/// How far strands turn away from straight out, in radians.
const SPREAD: f32 = 0.3;
/// How much a strand curls along its length, in radians.
const CURL: f32 = 0.6;
/// Strands that got kicked back onto the rug, and how hard they turn.
const FOLDED_CHANCE: f64 = 0.08;
const FOLD: f32 = 6.0;
/// Strands lie just above the floor, and above each other where they cross.
const LIFT: f32 = 0.0015;
const LAYER: f32 = 0.0003;

pub(crate) struct RugPlugin;

impl Plugin for RugPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Rug::on_add);
    }
}

/// A knotted rug on the floor, with loose strands at both ends.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Rug;

impl Rug {
    fn on_add(
        add: On<Add<Rug>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let mut rng = SmallRng::seed_from_u64(SEED);
        let fringe = meshes.add(Self::fringe(&mut rng));
        let wool = materials.add(StandardMaterial {
            base_color: WOOL,
            perceptual_roughness: 1.0,
            reflectance: 0.2,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        commands.entity(add.entity).with_children(|rug| {
            rug.spawn((Shape::cuboid(&Block::new(SIZE), RED), Finish::Rug));
            rug.spawn((Mesh3d(fringe), MeshMaterial3d(wool)));
        });
    }

    /// Flat strands of wool spilling out of both ends, each bent its own way
    /// and the neighbors of a tassel bent alike.
    fn fringe(rng: &mut impl Rng) -> Mesh {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let count = (SIZE.x / SPACING) as usize;
        let tassels = [rng.random_range(0.0..10.0), rng.random_range(0.0..10.0)];
        for (end, phase) in [-1.0, 1.0].into_iter().zip(tassels) {
            for index in 0..count {
                let x = -SIZE.x / 2.0
                    + SPACING * (index as f32 + 0.5)
                    + rng.random_range(-0.002..0.002);
                // Slow waves along the end, so neighbors lean together.
                let lean = (x * 9.0 + phase).sin() * 0.6 + (x * 23.0 + phase * 2.0).sin() * 0.4;
                let mut heading = lean * SPREAD + rng.random_range(-0.15..0.15);
                let mut curl = rng.random_range(-CURL..CURL);
                let length = rng.random_range(LENGTH);
                // Turns right round, partway along.
                let fold = rng
                    .random_bool(FOLDED_CHANCE)
                    .then(|| if rng.random_bool(0.5) { FOLD } else { -FOLD });
                let layer = (index % 3) as f32 * LAYER;
                let mut at = Vec2::new(x, end * SIZE.z / 2.0);
                let first = positions.len() as u32;
                for segment in 0..=SEGMENTS {
                    let along = segment as f32 / SEGMENTS as f32;
                    if let Some(fold) = fold
                        && along > 0.4
                    {
                        curl = fold;
                    }
                    let direction = Vec2::new(heading.sin(), end * heading.cos());
                    let side =
                        Vec2::new(direction.y, -direction.x) * WIDTH / 2.0 * (1.0 - 0.5 * along);
                    // Out of the knots, then down to the floor or over the rug.
                    let on_rug = at.y.abs() < SIZE.z / 2.0 - 0.002;
                    let height = if segment == 0 {
                        SIZE.y / 2.0
                    } else if on_rug {
                        SIZE.y + LIFT + layer
                    } else {
                        LIFT + layer
                    };
                    positions.push((at + side).extend(height).xzy());
                    positions.push((at - side).extend(height).xzy());
                    if segment < SEGMENTS {
                        let i = first + 2 * segment as u32;
                        indices.extend([i, i + 1, i + 2, i + 2, i + 1, i + 3]);
                    }
                    let step = length / SEGMENTS as f32;
                    at += direction * step;
                    heading += curl * step / length;
                }
            }
        }
        let normals = vec![Vec3::Y; positions.len()];
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
    }
}
