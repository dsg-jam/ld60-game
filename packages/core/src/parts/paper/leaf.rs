use std::f32::consts::{FRAC_PI_2, PI};

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

/// How far a turning page bends at most, in radians over its free width.
const CURL: f32 = 0.9;
/// Seconds a sheet takes to show its wrinkles once nothing presses it flat.
pub const RELAX_TIME: f32 = 0.45;
/// A turning leaf presses the sheet it comes down on flat once it is closer
/// than this many radians.
const CLEAR: f32 = FRAC_PI_2;

/// How the pages of a [`Paper`](super::Paper) are held together. The held
/// edge stays flat.
#[derive(Component, Clone, Debug)]
pub struct Binding {
    /// Points from the held edge across the page.
    across: Vec2,
}

impl Binding {
    /// Rings through the left edge, so the pages turn over to the left.
    pub fn rings() -> Self {
        Self { across: Vec2::X }
    }

    /// A clip on the top edge.
    pub fn clip() -> Self {
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
pub enum Stiffness {
    Paper,
    /// Thick card, like a divider, or pages held flat by one. It swings
    /// slower and does not bend.
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

/// The wrinkled sheet of one page: its front and back.
#[derive(Clone, Debug)]
pub struct LeafShape {
    size: Vec2,
    front: Mesh,
    back: Mesh,
}

impl LeafShape {
    pub fn new(wrinkles: &Wrinkles) -> Self {
        let front = wrinkles.mesh();
        let back = reversed(&front);
        Self {
            size: wrinkles.size(),
            front,
            back,
        }
    }

    /// The sheet lying with `share` of its wrinkles, face up like
    /// [`Paper::sheet`](super::Paper::sheet) or face down. Face down, the
    /// mesh turns over with the wrinkles bulging up from what it lies on
    /// instead of into it.
    pub fn lying(&self, face_up: bool, share: f32) -> Mesh {
        if face_up {
            relaxed(&self.front, share)
        } else {
            relaxed(&self.back, -share)
        }
    }
}

/// Which way the wrinkles of a side of a leaf may bulge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bulge {
    /// Up, whichever side is up, like a single page.
    Up,
    /// Only out of its front, with more pages behind it.
    Front,
    /// Only out of its back, with more pages in front of it.
    Back,
}

impl Bulge {
    /// The share of the wrinkles shown out of the front at `angle`.
    fn relax(self, angle: f32) -> f32 {
        let up = angle.cos();
        match self {
            Self::Up => up,
            Self::Front => up.max(0.0),
            Self::Back => up.min(0.0),
        }
    }
}

/// One side of a turning leaf.
pub struct LeafSide {
    pub shape: LeafShape,
    /// The back of the sheet instead of its front.
    pub back: bool,
    pub material: Handle<PaperMaterial>,
    /// The middle of the sheet lying face up, in the space of the leaf.
    pub at: Vec3,
    pub bulge: Bulge,
}

/// What turns over in one go.
pub struct LeafSpec {
    pub forward: bool,
    pub stiffness: Stiffness,
    /// How far from the axis of the leaf the sheet stays flat, where it is
    /// held.
    pub held: f32,
    /// Seconds since the sheet was last pressed flat, see [`relaxing`].
    pub relaxed: f32,
    pub sides: Vec<LeafSide>,
}

/// A leaf turning over, see [`LeafSpec`]. It despawns once it has turned.
///
/// Its owner places it each frame at [`Leaf::angle`]. In the space of the
/// leaf, X points across the sheet away from the held edge, Y out of its
/// front and Z along the axis it turns about. Its wrinkles relax as it
/// turns and it flattens as it comes down on the other stack.
#[derive(Component, Debug)]
pub struct Leaf {
    elapsed: f32,
    relaxed: f32,
    /// Whether it has made its sound yet. It sounds from where it is once
    /// it has a place in the world.
    sounded: bool,
    forward: bool,
    stiffness: Stiffness,
    sides: Vec<(Handle<Mesh>, Bend)>,
}

/// A side of the sheet lying flat in the space of the leaf.
#[derive(Clone, Debug)]
struct Bend {
    /// Where the sheet starts to bend.
    held: f32,
    /// From `held` to the free edge.
    width: f32,
    bulge: Bulge,
    /// Each vertex across the sheet, its height above the sheet and along
    /// the axis.
    points: Vec<Vec3>,
    /// How far the side lies out of the front of the leaf.
    lift: f32,
    /// The normal of each vertex, across, up and along the axis.
    normals: Vec<Vec3>,
}

impl Leaf {
    /// Spawns the leaf under `parent` at `at` with `meshes` for its sides,
    /// and returns it.
    pub fn spawn(
        commands: &mut Commands,
        (parent, at): (Entity, Transform),
        spec: LeafSpec,
        meshes: &mut Assets<Mesh>,
    ) -> Entity {
        let root = commands
            .spawn((at, Visibility::Inherited, ChildOf(parent)))
            .id();
        let mut sides = Vec::with_capacity(spec.sides.len());
        for (index, side) in spec.sides.into_iter().enumerate() {
            let flat = if side.back {
                &side.shape.back
            } else {
                &side.shape.front
            };
            let bend = Bend::new(flat, side.shape.size, side.at, (spec.held, side.bulge));
            let mesh = meshes.add(flat.clone());
            // It moves away from its bounds as it bends.
            let mut entity = commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(side.material),
                Transform::default(),
                Pickable::IGNORE,
                NoFrustumCulling,
                ChildOf(root),
            ));
            // Front and back lie on each other, so only one casts a shadow.
            if index > 0 {
                entity.insert(NotShadowCaster);
            }
            sides.push((mesh, bend));
        }
        let leaf = Leaf {
            elapsed: 0.0,
            relaxed: spec.relaxed,
            sounded: false,
            forward: spec.forward,
            stiffness: spec.stiffness,
            sides,
        };
        leaf.bend(meshes);
        commands.entity(root).insert(leaf);
        root
    }

    fn progress(&self) -> f32 {
        (self.elapsed / self.stiffness.time()).min(1.0)
    }

    /// How far the leaf has turned from lying to the right, in radians.
    pub fn angle(&self) -> f32 {
        let turned = self.stiffness.ease(self.progress()) * PI;
        if self.forward { turned } else { PI - turned }
    }

    /// The share of its wrinkles the leaf shows.
    fn wrinkles(&self) -> f32 {
        let left = if self.forward {
            PI - self.angle()
        } else {
            self.angle()
        };
        relaxing(self.relaxed).min(clearance(left))
    }

    /// Bends the sides of the sheet so the free edge lags behind the held
    /// edge.
    fn bend(&self, meshes: &mut Assets<Mesh>) {
        let lag = if self.forward { -1.0 } else { 1.0 };
        let curl = lag * self.stiffness.curl() * (self.progress() * PI).sin();
        let wrinkles = self.wrinkles();
        for (side, bend) in &self.sides {
            let Some(mut mesh) = meshes.get_mut(side) else {
                continue;
            };
            let (positions, normals) = bend.bent(curl, self.angle(), wrinkles);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        }
    }

    pub fn animate(
        time: Res<Time>,
        mut leaves: Query<(Entity, &mut Leaf, &GlobalTransform)>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut commands: Commands,
    ) {
        for (root, mut leaf, at) in &mut leaves {
            if !leaf.sounded {
                leaf.sounded = true;
                commands.trigger(PlayAt(leaf.stiffness.sound(), at.translation()));
                continue;
            }
            leaf.elapsed += time.delta_secs();
            leaf.relaxed += time.delta_secs();
            if leaf.progress() >= 1.0 {
                commands.entity(root).despawn();
                continue;
            }
            leaf.bend(&mut meshes);
        }
    }
}

impl Bend {
    /// `mesh` of a sheet of `size` with its middle at `at`, lying like
    /// [`Paper::sheet`](super::Paper::sheet). It bends from `held` on and
    /// its wrinkles `bulge`.
    fn new(mesh: &Mesh, size: Vec2, at: Vec3, (held, bulge): (f32, Bulge)) -> Self {
        let attribute = |id| match mesh.attribute(id) {
            Some(VertexAttributeValues::Float32x3(values)) => values.clone(),
            _ => Vec::new(),
        };
        // The mesh lies in its XY plane with the top of the page at +Y and
        // its height along Z.
        let points = attribute(Mesh::ATTRIBUTE_POSITION)
            .into_iter()
            .map(|[x, y, z]| Vec3::new(at.x + x, z, at.z - y))
            .collect();
        let normals = attribute(Mesh::ATTRIBUTE_NORMAL)
            .into_iter()
            .map(|[x, y, z]| Vec3::new(x, z, -y))
            .collect();
        Self {
            held,
            width: at.x + size.x / 2.0 - held,
            bulge,
            points,
            lift: at.y,
            normals,
        }
    }

    /// The positions and normals with the free part of the sheet bent
    /// evenly by `curl` radians, at `angle` from lying face up, with `share`
    /// of its wrinkles.
    fn bent(&self, curl: f32, angle: f32, share: f32) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
        let relax = self.bulge.relax(angle) * share;
        let bend = curl / self.width;
        let positions = self
            .points
            .iter()
            .map(|point| {
                let free = (point.x - self.held).max(0.0);
                let angle = bend * free;
                let along = if bend.abs() < 1e-4 {
                    Vec2::new(free, 0.0)
                } else {
                    Vec2::new(angle.sin(), 1.0 - angle.cos()) / bend
                };
                let up = Vec2::new(-angle.sin(), angle.cos());
                let at = Vec2::new(point.x.min(self.held), 0.0)
                    + along
                    + up * (point.y * relax + self.lift);
                [at.x, at.y, point.z]
            })
            .collect();
        let normals = self
            .points
            .iter()
            .zip(&self.normals)
            .map(|(point, normal)| {
                let relaxed = Vec3::new(normal.x * relax, normal.y, normal.z * relax).normalize();
                let free = (point.x - self.held).max(0.0);
                let turned = Vec2::from_angle(bend * free).rotate(relaxed.xy());
                turned.extend(relaxed.z).to_array()
            })
            .collect();
        (positions, normals)
    }
}

/// The share of its wrinkles a sheet shows `elapsed` seconds after it was
/// last pressed flat, from 0 to 1.
pub fn relaxing(elapsed: f32) -> f32 {
    smoothstep(elapsed / RELAX_TIME)
}

/// The share of its wrinkles the top sheet of a stack shows under a leaf
/// turning `angle` radians away from it.
pub fn clearance(angle: f32) -> f32 {
    smoothstep(angle / CLEAR)
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `material` with `print` on it instead.
pub fn reprinted(material: &PaperMaterial, print: Handle<Image>) -> PaperMaterial {
    ExtendedMaterial {
        base: material.base.clone(),
        extension: material.extension.reprinted(print),
    }
}

/// A print with nothing on it, for the back of a page.
pub fn blank_print(images: &mut Assets<Image>) -> Handle<Image> {
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
pub(super) fn reversed(mesh: &Mesh) -> Mesh {
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

/// `mesh` of a sheet with its wrinkles scaled by `relax`, see
/// [`Bulge::relax`].
pub(super) fn relaxed(mesh: &Mesh, relax: f32) -> Mesh {
    let mut relaxed = mesh.clone();
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        relaxed.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for position in positions {
            position[2] *= relax;
        }
    }
    if let Some(VertexAttributeValues::Float32x3(normals)) =
        relaxed.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
    {
        for normal in normals {
            *normal = Vec3::new(normal[0] * relax, normal[1] * relax, normal[2])
                .normalize()
                .to_array();
        }
    }
    relaxed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bend(bulge: Bulge) -> Bend {
        let wrinkles = Wrinkles::new(Vec2::new(0.16, 0.22), 7);
        Bend::new(
            &wrinkles.mesh(),
            wrinkles.size(),
            Vec3::new(0.074, 0.0, 0.0),
            (0.006, bulge),
        )
    }

    #[test]
    fn the_held_strip_stays_put_while_the_leaf_bends() {
        let bend = bend(Bulge::Up);
        let (flat, _) = bend.bent(0.0, 0.0, 1.0);
        let (bent, _) = bend.bent(0.8, 0.0, 1.0);
        for ((point, flat), bent) in bend.points.iter().zip(&flat).zip(&bent) {
            if point.x <= bend.held {
                assert_eq!(flat, bent);
            }
        }
    }

    #[test]
    fn a_leaf_lies_flat_as_it_lands() {
        let mut leaf = Leaf {
            elapsed: 0.0,
            relaxed: 1.0,
            sounded: true,
            forward: true,
            stiffness: Stiffness::Paper,
            sides: Vec::new(),
        };
        assert!((leaf.wrinkles() - 1.0).abs() < 1e-6);
        leaf.elapsed = Stiffness::Paper.time();
        assert!(leaf.wrinkles().abs() < 1e-6);
        let (flat, _) = bend(Bulge::Up).bent(0.0, PI, 0.0);
        assert!(flat.iter().all(|p| p[1].abs() < 1e-6));
    }

    #[test]
    fn wrinkles_bulge_away_from_the_pages_behind() {
        let front = bend(Bulge::Front);
        let back = bend(Bulge::Back);
        for angle in [0.0, 1.0, 2.0, PI] {
            let (front, _) = front.bent(0.0, angle, 1.0);
            let (back, _) = back.bent(0.0, angle, 1.0);
            assert!(front.iter().all(|p| p[1] >= -1e-6));
            assert!(back.iter().all(|p| p[1] <= 1e-6));
        }
    }
}
