use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    light::NotShadowCaster,
    mesh::{Indices, VertexAttributeValues},
    pbr::ExtendedMaterial,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::sheet::{PaperMaterial, Wrinkles};
use crate::audio::{PlayAt, Sound};

/// How far a turning page bends at most, in radians over its width.
const CURL: f32 = 0.9;

/// How the pages of a [`Paper`](super::Paper) are held together. The held
/// edge stays flat.
#[derive(Component, Clone, Debug)]
pub(crate) struct Binding {
    /// Points from the held edge across the page.
    across: Vec2,
}

impl Binding {
    /// Rings through the left edge, so the pages turn over to the left.
    pub(crate) fn rings() -> Self {
        Self { across: Vec2::X }
    }

    /// A clip on the top edge.
    pub(crate) fn clip() -> Self {
        Self {
            across: Vec2::NEG_Y,
        }
    }

    /// Points from the middle of the sheet to the edge the binding grips.
    pub(super) fn held(&self) -> Vec2 {
        -self.across
    }

    /// Whether `point` on the sheet, from -0.5 to 0.5 with the top at +0.5,
    /// lies past the held edge, where the turned pages are.
    pub(super) fn beyond(&self, point: Vec2) -> bool {
        (point - self.held() * 0.5).dot(self.across) < 0.0
    }
}

/// How a leaf bends and swings as it turns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stiffness {
    Paper,
    /// Thick card, like a divider. It swings slower and does not bend.
    Card,
}

impl Stiffness {
    /// Seconds to turn over.
    fn time(self) -> f32 {
        match self {
            Self::Paper => 0.3,
            Self::Card => 0.38,
        }
    }

    fn curl(self) -> f32 {
        match self {
            Self::Paper => CURL,
            Self::Card => 0.0,
        }
    }

    /// How far it has turned after `t` of its time.
    fn ease(self, t: f32) -> f32 {
        match self {
            Self::Paper => t * t * (3.0 - 2.0 * t),
            Self::Card => t * t * t * (t * (6.0 * t - 15.0) + 10.0),
        }
    }

    fn sound(self) -> Sound {
        match self {
            Self::Paper => Sound::PageTurn,
            Self::Card => Sound::DividerTurn,
        }
    }
}

/// The wrinkled sheet that bends as it turns, and its back for a sheet
/// lying face down.
#[derive(Clone, Debug)]
pub(crate) struct LeafShape {
    size: Vec2,
    /// How high the wrinkles rise above the flat sheet.
    rise: f32,
    front: Mesh,
    back: Mesh,
    back_handle: Handle<Mesh>,
}

impl LeafShape {
    pub(crate) fn new(wrinkles: &Wrinkles, meshes: &mut Assets<Mesh>) -> Self {
        let front = wrinkles.mesh();
        let back = reversed(&front);
        Self {
            size: wrinkles.size(),
            rise: highest(&front),
            back_handle: meshes.add(back.clone()),
            front,
            back,
        }
    }

    /// The back of the whole sheet, seen from below the sheet's front.
    pub(crate) fn back(&self) -> Handle<Mesh> {
        self.back_handle.clone()
    }

    /// How much higher a sheet lies face down than face up, so its wrinkles
    /// bulge up from what it lies on instead of into it.
    pub(crate) fn rise(&self) -> f32 {
        self.rise
    }
}

/// What turns over in one go: the sheet on top and anything that turns
/// with it.
pub(crate) struct LeafSpec {
    /// The leaf turns about the Z axis of this. Forward is a half turn the
    /// right way round, so a sheet lying to the right of the axis ends up
    /// face down on the left.
    pub(crate) pivot: Transform,
    /// The middle of the sheet lying face up to the right, before turning
    /// forward, relative to `pivot`. Its top points to -Z, like
    /// [`Paper::sheet`](super::Paper::sheet).
    pub(crate) at: Vec3,
    pub(crate) forward: bool,
    pub(crate) stiffness: Stiffness,
    pub(crate) front: Handle<PaperMaterial>,
    /// Shows under the sheet, unless something below covers it.
    pub(crate) back: Option<Handle<PaperMaterial>>,
    /// Turn along without bending, each relative to `pivot` before turning
    /// forward.
    pub(crate) riders: Vec<(Entity, Transform)>,
}

/// A leaf turning over, see [`LeafSpec`]. It despawns once it lies flat
/// again and hands its riders back to its parent.
#[derive(Component, Debug)]
pub(crate) struct Leaf {
    elapsed: f32,
    /// Whether it has made its sound yet. It sounds from where it is once
    /// it has a place in the world.
    sounded: bool,
    forward: bool,
    stiffness: Stiffness,
    pivot: Transform,
    rise: f32,
    riders: Vec<(Entity, Transform)>,
    /// The flat sheet, which [`Leaf::bend`] bends.
    flat: Bend,
    /// The meshes of the front and the back, bent each frame.
    sides: Vec<Handle<Mesh>>,
}

/// Where the sheet of a leaf lies flat, relative to its root.
#[derive(Clone, Debug)]
struct Bend {
    /// The near edge, where the sheet starts to bend.
    edge: Vec2,
    width: f32,
    /// Each vertex of the sheet mesh, as the distance from the near edge,
    /// the height above the sheet and the position along the axis.
    points: Vec<Vec3>,
    /// The normal of each vertex, across, up and along the axis.
    normals: Vec<Vec3>,
}

impl Leaf {
    /// Spawns the leaf under `parent` with `meshes` for its sides, and
    /// returns it.
    pub(crate) fn spawn(
        commands: &mut Commands,
        parent: Entity,
        shape: &LeafShape,
        spec: LeafSpec,
        meshes: &mut Assets<Mesh>,
    ) -> Entity {
        let mut leaf = Leaf {
            elapsed: 0.0,
            sounded: false,
            forward: spec.forward,
            stiffness: spec.stiffness,
            pivot: spec.pivot,
            rise: if spec.riders.is_empty() {
                shape.rise
            } else {
                0.0
            },
            riders: spec.riders,
            flat: Bend::new(&shape.front, shape.size, spec.at),
            sides: Vec::with_capacity(2),
        };
        let root = commands
            .spawn((leaf.root(), Visibility::Inherited, ChildOf(parent)))
            .id();
        for &(rider, at) in &leaf.riders {
            commands.entity(rider).insert((at, ChildOf(root)));
        }
        let sides = [(&shape.front, Some(spec.front)), (&shape.back, spec.back)];
        for (mesh, material) in sides {
            let Some(material) = material else {
                continue;
            };
            let mesh = meshes.add(mesh.clone());
            // Front and back lie on each other, so their shadows would fall
            // on each other in stripes. It moves away from its bounds as it
            // bends.
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                Transform::default(),
                Pickable::IGNORE,
                NotShadowCaster,
                NoFrustumCulling,
                ChildOf(root),
            ));
            leaf.sides.push(mesh);
        }
        leaf.bend(meshes);
        commands.entity(root).insert(leaf);
        root
    }

    fn progress(&self) -> f32 {
        (self.elapsed / self.stiffness.time()).min(1.0)
    }

    /// How far the leaf has turned from lying to the right, in radians.
    fn angle(&self) -> f32 {
        let turned = self.stiffness.ease(self.progress()) * PI;
        if self.forward { turned } else { PI - turned }
    }

    /// Turns about the pivot, and rises as it goes over, see
    /// [`LeafShape::rise`].
    fn root(&self) -> Transform {
        let angle = self.angle();
        Transform::from_translation(Vec3::Y * self.rise * angle / PI)
            * self.pivot
            * Transform::from_rotation(Quat::from_rotation_z(angle))
    }

    /// Bends the sides of the sheet so the free edge lags behind the edge
    /// at the axis.
    fn bend(&self, meshes: &mut Assets<Mesh>) {
        let lag = if self.forward { -1.0 } else { 1.0 };
        let curl = lag * self.stiffness.curl() * (self.progress() * PI).sin();
        let (positions, normals) = self.flat.bent(curl);
        for (index, side) in self.sides.iter().enumerate() {
            let Some(mut mesh) = meshes.get_mut(side) else {
                continue;
            };
            let sign = if index == 0 { 1.0 } else { -1.0 };
            let normals: Vec<[f32; 3]> = normals.iter().map(|n| (*n * sign).to_array()).collect();
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        }
    }

    pub(crate) fn animate(
        time: Res<Time>,
        mut leaves: Query<(Entity, &mut Leaf, &ChildOf, &GlobalTransform)>,
        mut transforms: Query<&mut Transform>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut commands: Commands,
    ) {
        for (root, mut leaf, parent, at) in &mut leaves {
            if !leaf.sounded {
                leaf.sounded = true;
                commands.trigger(PlayAt(leaf.stiffness.sound(), at.translation()));
                continue;
            }
            leaf.elapsed += time.delta_secs();
            let at = leaf.root();
            if let Ok(mut transform) = transforms.get_mut(root) {
                *transform = at;
            }
            if leaf.progress() >= 1.0 {
                for &(rider, relative) in &leaf.riders {
                    commands
                        .entity(rider)
                        .insert((at * relative, ChildOf(parent.parent())));
                }
                commands.entity(root).despawn();
                continue;
            }
            leaf.bend(&mut meshes);
        }
    }
}

impl Bend {
    /// `mesh` of a sheet of `size` with its middle at `at`, lying like
    /// [`Paper::sheet`](super::Paper::sheet).
    fn new(mesh: &Mesh, size: Vec2, at: Vec3) -> Self {
        let attribute = |id| match mesh.attribute(id) {
            Some(VertexAttributeValues::Float32x3(values)) => values.clone(),
            _ => Vec::new(),
        };
        // The mesh lies in its XY plane with the top of the page at +Y and
        // its height along Z.
        let points = attribute(Mesh::ATTRIBUTE_POSITION)
            .into_iter()
            .map(|[x, y, z]| Vec3::new(x + size.x / 2.0, z, at.z - y))
            .collect();
        let normals = attribute(Mesh::ATTRIBUTE_NORMAL)
            .into_iter()
            .map(|[x, y, z]| Vec3::new(x, z, -y))
            .collect();
        Self {
            edge: Vec2::new(at.x - size.x / 2.0, at.y),
            width: size.x,
            points,
            normals,
        }
    }

    /// The positions and normals with the sheet bent evenly by `curl`
    /// radians from the near edge to the free edge.
    fn bent(&self, curl: f32) -> (Vec<[f32; 3]>, Vec<Vec3>) {
        let bend = curl / self.width;
        let positions = self
            .points
            .iter()
            .map(|point| {
                let angle = bend * point.x;
                let along = if bend.abs() < 1e-4 {
                    Vec2::new(point.x, 0.0)
                } else {
                    Vec2::new(angle.sin(), 1.0 - angle.cos()) / bend
                };
                let up = Vec2::new(-angle.sin(), angle.cos());
                let at = self.edge + along + up * point.y;
                [at.x, at.y, point.z]
            })
            .collect();
        let normals = self
            .points
            .iter()
            .zip(&self.normals)
            .map(|(point, normal)| {
                let turned = Vec2::from_angle(bend * point.x).rotate(normal.xy());
                turned.extend(normal.z)
            })
            .collect();
        (positions, normals)
    }
}

/// `material` with `print` on it instead.
pub(crate) fn reprinted(material: &PaperMaterial, print: Handle<Image>) -> PaperMaterial {
    ExtendedMaterial {
        base: material.base.clone(),
        extension: material.extension.reprinted(print),
    }
}

/// A print with nothing on it, for the back of a page.
pub(crate) fn blank_print(images: &mut Assets<Image>) -> Handle<Image> {
    images.add(Image::new_fill(
        Extent3d::default(),
        TextureDimension::D2,
        &[255; 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ))
}

/// `mesh` seen from behind: the same triangles wound the other way round,
/// with the normals flipped.
fn reversed(mesh: &Mesh) -> Mesh {
    let mut back = mesh.clone();
    if let Some(VertexAttributeValues::Float32x3(normals)) =
        back.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
    {
        for normal in normals {
            *normal = normal.map(|n| -n);
        }
    }
    if let Some(Indices::U32(indices)) = back.indices_mut() {
        for triangle in indices.as_chunks_mut::<3>().0 {
            triangle.swap(1, 2);
        }
    }
    back
}

/// The height of the highest point of `mesh`.
fn highest(mesh: &Mesh) -> f32 {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(positions)) => positions
            .iter()
            .map(|position| position[2])
            .fold(0.0, f32::max),
        _ => 0.0,
    }
}
