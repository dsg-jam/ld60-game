use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use super::block::Block;

/// A slab with rectangular holes through it, like a panel with openings
/// for the parts that sit in it. The holes lie inside the slab and don't
/// overlap.
pub(crate) struct Frame<'a> {
    pub(crate) outer: Block,
    /// Each hole's walls reach back from the front of `outer` to the back
    /// of the hole's block, at most to the back of `outer`. A hole whose
    /// block starts at the front or beyond has no walls, for parts that
    /// fill it.
    pub(crate) holes: &'a [Block],
}

impl Frame<'_> {
    /// The front and sides, as one mesh so no seams show.
    pub(crate) fn mesh(&self) -> Mesh {
        let (outer, holes) = (&self.outer, self.holes);
        let (low, high) = (outer.min(), outer.max());
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        // Four corners in order around the quad, and the way it faces.
        let mut quad = |corners: [Vec3; 4], normal: Vec3| {
            let first = positions.len() as u32;
            positions.extend(corners);
            normals.extend([normal; 4]);
            let facing = (corners[1] - corners[0])
                .cross(corners[2] - corners[0])
                .dot(normal);
            let order = if facing > 0.0 {
                [0, 1, 2, 0, 2, 3]
            } else {
                [0, 2, 1, 0, 3, 2]
            };
            indices.extend(order.map(|index| first + index));
        };
        // The front, cut into a grid along the edges of the holes, without the
        // cells inside them.
        let mut xs = vec![low.x, high.x];
        let mut ys = vec![low.y, high.y];
        for hole in holes {
            xs.extend([hole.min().x, hole.max().x]);
            ys.extend([hole.min().y, hole.max().y]);
        }
        for line in [&mut xs, &mut ys] {
            line.sort_by(f32::total_cmp);
            line.dedup();
        }
        for x in xs.windows(2) {
            for y in ys.windows(2) {
                let middle = Vec2::new(x[0] + x[1], y[0] + y[1]) / 2.0;
                let open = holes.iter().any(|hole| {
                    middle.cmpgt(hole.min().truncate()).all()
                        && middle.cmplt(hole.max().truncate()).all()
                });
                if !open {
                    let z = high.z;
                    quad(
                        [
                            Vec3::new(x[0], y[0], z),
                            Vec3::new(x[1], y[0], z),
                            Vec3::new(x[1], y[1], z),
                            Vec3::new(x[0], y[1], z),
                        ],
                        Vec3::Z,
                    );
                }
            }
        }
        // Walls facing out around the edge, and in around each hole.
        let walls = |a: Vec2, b: Vec2| {
            [
                ([a.extend(low.z), Vec3::new(b.x, a.y, low.z)], Vec3::NEG_Y),
                ([Vec3::new(b.x, a.y, low.z), b.extend(low.z)], Vec3::X),
                ([b.extend(low.z), Vec3::new(a.x, b.y, low.z)], Vec3::Y),
                ([Vec3::new(a.x, b.y, low.z), a.extend(low.z)], Vec3::NEG_X),
            ]
        };
        for ([from, to], normal) in walls(low.truncate(), high.truncate()) {
            quad([from, to, to.with_z(high.z), from.with_z(high.z)], normal);
        }
        for hole in holes {
            let floor = hole.min().z.max(low.z);
            if floor >= high.z {
                continue;
            }
            for ([from, to], normal) in walls(hole.min().truncate(), hole.max().truncate()) {
                let (from, to) = (from.with_z(floor), to.with_z(floor));
                quad([from, to, to.with_z(high.z), from.with_z(high.z)], -normal);
            }
        }
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
    }
}
