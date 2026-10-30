mod drilled;
mod keycap;

use bevy::{mesh::VertexAttributeValues, prelude::*};

use super::bar::Bar;
use super::block::Block;
use super::surface::{Finish, Form, SurfaceMaterial, SurfacePlugin};

/// Shapes get one of this many seeds, which vary their surfaces.
const SEEDS: u32 = 997;

pub struct ShapePlugin;

impl Plugin for ShapePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SurfacePlugin).add_observer(Shape::on_add);
    }
}

/// A primitive mesh in one color, placed by a [`Block`].
///
/// Add a [`Finish`] next to it for anything other than paint.
#[derive(Component, Clone, Debug)]
#[require(Transform)]
pub struct Shape {
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

#[derive(Clone, Debug)]
enum Kind {
    Cuboid,
    /// Upright cylinder that fills the width and height of its block.
    Cylinder,
    /// Like [`Kind::Cylinder`], with round ends inside its block.
    Capsule,
    /// A ring lying flat, as wide as its block and as thick as it is high.
    Ring,
    /// A round mesh of its own that fills its block.
    Turned(Handle<Mesh>),
    /// Like [`Kind::Cuboid`], with round holes of a radius through it from
    /// top to bottom.
    Drilled(Vec<Vec2>, f32),
    /// A keycap, with sides that lean in by a taper and a top dished by a
    /// depth.
    Keycap(f32, f32),
}

impl Shape {
    pub fn cuboid(block: &Block, color: Color) -> impl Bundle {
        Self::bundle(Kind::Cuboid, block, color)
    }

    /// A cuboid filling `bar`.
    pub fn bar(bar: &Bar, color: Color) -> impl Bundle {
        (
            Self {
                kind: Kind::Cuboid,
                size: bar.size(),
                color,
                texture: None,
            },
            bar.placement(),
        )
    }

    pub fn cylinder(block: &Block, color: Color) -> impl Bundle {
        Self::bundle(Kind::Cylinder, block, color)
    }

    pub fn capsule(block: &Block, color: Color) -> impl Bundle {
        Self::bundle(Kind::Capsule, block, color)
    }

    /// A ring lying flat in `block`, like a coil of cable.
    pub fn ring(block: &Block, color: Color) -> impl Bundle {
        Self::bundle(Kind::Ring, block, color)
    }

    /// A keycap filling `block`, with sides that lean in by `taper` and a
    /// top dished by `dish`, showing `region` of `image` on its top. With
    /// both at zero it is a plain box.
    pub fn keycap(
        block: &Block,
        (taper, dish): (f32, f32),
        image: Handle<Image>,
        region: Rect,
    ) -> impl Bundle {
        (
            Self {
                kind: if (taper, dish) == (0.0, 0.0) {
                    Kind::Cuboid
                } else {
                    Kind::Keycap(taper, dish)
                },
                size: block.size,
                color: Color::WHITE,
                texture: Some(TopTexture { image, region }),
            },
            block.transform(),
        )
    }

    /// A cuboid with round holes of `radius` through it from top to bottom.
    /// `holes` are on its top, in meters from its middle.
    pub fn drilled(block: &Block, holes: &[Vec2], radius: f32, color: Color) -> impl Bundle {
        Self::bundle(Kind::Drilled(holes.to_vec(), radius), block, color)
    }

    /// A round part with its own `mesh`, which fills `block` and is
    /// finished like a cylinder.
    pub fn turned(mesh: Handle<Mesh>, block: &Block, color: Color) -> impl Bundle {
        Self::bundle(Kind::Turned(mesh), block, color)
    }

    /// A cuboid with `region` of `image` on its top face.
    pub fn textured_cuboid(block: &Block, image: Handle<Image>, region: Rect) -> impl Bundle {
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

    fn mesh(&self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        let mut mesh: Mesh = match &self.kind {
            Kind::Cuboid => Cuboid::from_size(self.size).into(),
            Kind::Cylinder => Cylinder::new(self.size.x / 2.0, self.size.y).into(),
            Kind::Capsule => Capsule3d::new(self.size.x / 2.0, self.size.y - self.size.x).into(),
            Kind::Ring => {
                let outer = self.size.x / 2.0;
                Torus::new(outer - self.size.y, outer)
                    .mesh()
                    .minor_resolution(8)
                    .major_resolution(24)
                    .build()
            }
            Kind::Turned(mesh) => return mesh.clone(),
            Kind::Drilled(holes, radius) => drilled::drilled(self.size, holes, *radius),
            Kind::Keycap(taper, dish) => keycap::keycap(self.size, *taper, *dish),
        };
        if let Some(texture) = &self.texture {
            texture.map_uvs(&mut mesh);
        }
        meshes.add(mesh)
    }

    fn on_add(
        add: On<Add<Shape>>,
        shapes: Query<(&Shape, Option<&Finish>)>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let Ok((shape, finish)) = shapes.get(add.entity) else {
            return;
        };
        let form = match shape.kind {
            Kind::Cuboid | Kind::Drilled(..) | Kind::Keycap(..) => Form::Box(shape.size),
            Kind::Cylinder | Kind::Capsule | Kind::Ring | Kind::Turned(_) => {
                Form::Cylinder(shape.size)
            }
        };
        let seed = (add.entity.index_u32() % SEEDS) as f32;
        commands.entity(add.entity).insert((
            Mesh3d(shape.mesh(&mut meshes)),
            MeshMaterial3d(materials.add(finish.copied().unwrap_or_default().textured(
                shape.color,
                shape.texture.as_ref().map(|t| t.image.clone()),
                form,
                seed,
            ))),
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
