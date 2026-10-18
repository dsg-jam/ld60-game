use bevy::{mesh::VertexAttributeValues, prelude::*};

use super::block::Block;

pub(crate) struct ShapePlugin;

impl Plugin for ShapePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Shape::on_add);
    }
}

/// A primitive mesh with a plain color, placed by a [`Block`].
#[derive(Component, Clone, Debug)]
#[require(Transform)]
pub(crate) struct Shape {
    kind: Kind,
    size: Vec3,
    color: Color,
    texture: Option<TopTexture>,
}

/// An image region shown on the top face.
///
/// Other faces take the color at the corner of the region.
#[derive(Clone, Debug)]
struct TopTexture {
    image: Handle<Image>,
    region: Rect,
}

#[derive(Clone, Copy, Debug)]
enum Kind {
    Cuboid,
    /// Upright cylinder that fills the width and height of its block.
    Cylinder,
}

impl Shape {
    pub(crate) fn cuboid(block: &Block, color: Color) -> impl Bundle {
        Self::bundle(Kind::Cuboid, block, color)
    }

    pub(crate) fn cylinder(block: &Block, color: Color) -> impl Bundle {
        Self::bundle(Kind::Cylinder, block, color)
    }

    /// A cuboid with `region` of `image` on its top face.
    pub(crate) fn textured_cuboid(
        block: &Block,
        image: Handle<Image>,
        region: Rect,
    ) -> impl Bundle {
        (
            Self {
                kind: Kind::Cuboid,
                size: block.size,
                color: Color::WHITE,
                texture: Some(TopTexture { image, region }),
            },
            block.transform(),
        )
    }

    fn bundle(kind: Kind, block: &Block, color: Color) -> impl Bundle {
        (
            Self {
                kind,
                size: block.size,
                color,
                texture: None,
            },
            block.transform(),
        )
    }

    fn mesh(&self) -> Mesh {
        let mut mesh: Mesh = match self.kind {
            Kind::Cuboid => Cuboid::from_size(self.size).into(),
            Kind::Cylinder => Cylinder::new(self.size.x / 2.0, self.size.y).into(),
        };
        if let Some(texture) = &self.texture {
            texture.map_uvs(&mut mesh);
        }
        mesh
    }

    fn on_add(
        add: On<Add<Shape>>,
        shapes: Query<&Shape>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok(shape) = shapes.get(add.entity) else {
            return;
        };
        commands.entity(add.entity).insert((
            Mesh3d(meshes.add(shape.mesh())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: shape.color,
                base_color_texture: shape.texture.as_ref().map(|t| t.image.clone()),
                perceptual_roughness: 0.8,
                ..default()
            })),
        ));
    }
}

impl TopTexture {
    fn map_uvs(&self, mesh: &mut Mesh) {
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            return;
        };
        let tops: Vec<bool> = normals.iter().map(|normal| normal[1] > 0.5).collect();
        let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
        else {
            return;
        };
        let corner = self.region.min + self.region.size() * 0.05;
        for (uv, top) in uvs.iter_mut().zip(tops) {
            let mapped = if top {
                self.region.min + Vec2::from(*uv) * self.region.size()
            } else {
                corner
            };
            *uv = mapped.to_array();
        }
    }
}
