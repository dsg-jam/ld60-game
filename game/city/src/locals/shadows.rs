use std::f32::consts::PI;

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use ld60_game_player::camera::PlayerCamera;
use ld60_geometry::quads::Quads;

/// The most people with a shadow at once, the nearest to the player. Matches
/// `shadows.wesl`.
const CASTERS: usize = 40;
/// Lights per shadow, the strongest at the feet, and the sky. Matches
/// `shadows.wesl`.
const LIGHTS: usize = 3;
const BLOBS: usize = 3;
const BONES: usize = 7;
/// People cast shadows within this distance of the player, from lights
/// within the other.
const CAST_RANGE: f32 = 26.0;
const LIGHT_RANGE: f32 = 45.0;
/// Light from the sky and the windows, in lux, that no shadow takes away.
const AMBIENT: f32 = 25.0;
/// How dark the shadow of a light can be at most, and how dark the sky's
/// shadow is right under someone.
const DARKEST: f32 = 0.85;
const SKY: f32 = 0.35;
/// The sky light, far above and very wide.
const SKY_HEIGHT: f32 = 12.0;
const SKY_SIZE: f32 = 14.0;
/// Size of a lamp, which softens its shadows.
const LAMP_SIZE: f32 = 0.12;
/// The longest shadow drawn, from the feet.
const LONGEST: f32 = 5.0;
/// Room around the shapes for the soft edges.
const MARGIN: f32 = 0.6;

pub(super) struct ShadowPlugin;

impl Plugin for ShadowPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shadows.wesl");
        app.add_plugins(MaterialPlugin::<ShadowMaterial>::default())
            .add_systems(Startup, ShadowMaterial::spawn)
            .init_resource::<Lamps>()
            .add_systems(PostUpdate, (Lamps::gather, ShadowMaterial::cast).chain());
    }
}

/// The shapes that make someone's shadow, in world space: ellipsoids for
/// the round parts and capsules for the thin ones. Unused shapes have no
/// size.
#[derive(Component, Debug, Default, Clone)]
pub(super) struct Silhouette {
    pub(super) feet: Vec3,
    /// Height of the highest point above the feet.
    pub(super) height: f32,
    pub(super) blobs: [Blob; BLOBS],
    pub(super) bones: [Bone; BONES],
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Blob {
    pub(super) center: Vec3,
    /// The half axes.
    pub(super) axes: [Vec3; 3],
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Bone {
    pub(super) from: Vec3,
    pub(super) to: Vec3,
    pub(super) radius: f32,
}

impl Blob {
    /// An ellipsoid of `scale` across, turned and placed by `transform`.
    pub(super) fn new(transform: &Transform, scale: Vec3) -> Self {
        let half = scale / 2.0;
        Self {
            center: transform.translation,
            axes: [
                transform.rotation * Vec3::X * half.x,
                transform.rotation * Vec3::Y * half.y,
                transform.rotation * Vec3::Z * half.z,
            ],
        }
    }

    fn moved(self, by: &Transform) -> Self {
        Self {
            center: by.transform_point(self.center),
            axes: self.axes.map(|axis| by.rotation * axis),
        }
    }

    /// The center with the mean radius, then the rows that take a point
    /// into the space where the ellipsoid is a unit sphere.
    fn gpu(&self) -> [Vec4; 4] {
        let lengths = self.axes.map(Vec3::length_squared);
        if lengths.contains(&0.0) {
            return [Vec4::ZERO; 4];
        }
        let mean = lengths.iter().product::<f32>().powf(1.0 / 6.0);
        let [x, y, z] = [0, 1, 2].map(|axis| (self.axes[axis] / lengths[axis]).extend(0.0));
        [self.center.extend(mean), x, y, z]
    }
}

impl Bone {
    fn moved(self, by: &Transform) -> Self {
        Self {
            from: by.transform_point(self.from),
            to: by.transform_point(self.to),
            radius: self.radius,
        }
    }
}

impl Silhouette {
    /// This silhouette, made in the space of `by`, in world space.
    pub(super) fn moved(&self, by: &Transform) -> Self {
        Self {
            feet: by.transform_point(self.feet),
            height: self.height,
            blobs: self.blobs.map(|blob| blob.moved(by)),
            bones: self.bones.map(|bone| bone.moved(by)),
        }
    }
}

/// The lights near the player.
#[derive(Resource, Debug, Default)]
pub(super) struct Lamps(Vec<Lamp>);

impl Lamps {
    /// The light falling on `at` from all the lamps, in lux per color.
    pub(super) fn light_at(&self, at: Vec3) -> Vec3 {
        self.0.iter().map(|lamp| lamp.color * lamp.lux_at(at)).sum()
    }

    fn gather(
        camera: Single<&GlobalTransform, With<PlayerCamera>>,
        spots: Query<(&SpotLight, &GlobalTransform, &InheritedVisibility)>,
        points: Query<(&PointLight, &GlobalTransform, &InheritedVisibility)>,
        mut lamps: ResMut<Self>,
    ) {
        let eye = camera.translation();
        let spots = spots
            .iter()
            .filter(|(_, _, visible)| visible.get())
            .map(|(light, at, _)| {
                let scale = 1.0 / (light.inner_angle.cos() - light.outer_angle.cos()).max(1e-4);
                Lamp {
                    at: at.translation(),
                    intensity: light.intensity / (4.0 * PI),
                    range: light.range,
                    color: light.color.to_linear().to_vec3(),
                    spot: Some((at.forward().into(), scale, -light.outer_angle.cos() * scale)),
                }
            });
        let points = points
            .iter()
            .filter(|(_, _, visible)| visible.get())
            .map(|(light, at, _)| Lamp {
                at: at.translation(),
                intensity: light.intensity / (4.0 * PI),
                range: light.range,
                color: light.color.to_linear().to_vec3(),
                spot: None,
            });
        lamps.0.clear();
        lamps.0.extend(
            spots
                .chain(points)
                .filter(|lamp| lamp.at.distance(eye) < LIGHT_RANGE),
        );
    }
}

/// A light near the people, as seen by their shadows.
#[derive(Debug, Clone, Copy)]
struct Lamp {
    at: Vec3,
    /// Lumens per steradian.
    intensity: f32,
    range: f32,
    color: Vec3,
    /// Where a spot light points, and its cone as Bevy fades it.
    spot: Option<(Vec3, f32, f32)>,
}

impl Lamp {
    /// Lux at `at`, from any side.
    fn lux_at(&self, at: Vec3) -> f32 {
        let to = self.at - at;
        let distance_squared = to.length_squared().max(0.01);
        let window = (1.0 - (distance_squared / (self.range * self.range)).powi(2))
            .clamp(0.0, 1.0)
            .powi(2);
        let cone = self.spot.map_or(1.0, |(direction, scale, offset)| {
            let along = direction.dot(-to.normalize());
            (along * scale + offset).clamp(0.0, 1.0).powi(2)
        });
        self.intensity / distance_squared * window * cone
    }

    /// Lux on the ground at `at`.
    fn lux(&self, at: Vec3) -> f32 {
        let to = self.at - at;
        let distance_squared = to.length_squared().max(0.01);
        let window = (1.0 - (distance_squared / (self.range * self.range)).powi(2))
            .clamp(0.0, 1.0)
            .powi(2);
        let facing = (to.y / distance_squared.sqrt()).max(0.0);
        let cone = self.spot.map_or(1.0, |(direction, scale, offset)| {
            let along = direction.dot(-to.normalize());
            (along * scale + offset).clamp(0.0, 1.0).powi(2)
        });
        self.intensity / distance_squared * window * facing * cone
    }
}

/// Soft shadows of people on the ground, traced in the shader from the
/// shapes of their bodies to the strongest lights near them.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct ShadowMaterial {
    #[uniform(0)]
    casters: Casters,
}

/// Matches `Casters` in `shadows.wesl`.
#[derive(ShaderType, Debug, Clone)]
struct Casters {
    casters: [Caster; CASTERS],
}

/// Matches `Caster` in `shadows.wesl`.
#[derive(ShaderType, Debug, Clone, Copy, Default)]
struct Caster {
    /// The ground under the shadow, from the lowest X and Z to the highest.
    bounds: Vec4,
    /// The height of the ground, in X.
    ground: Vec4,
    /// Where each light is, with its size, and how much of the light on
    /// the ground it brings, in each color.
    lights: [Vec4; LIGHTS],
    tints: [Vec4; LIGHTS],
    blobs: [Vec4; 4 * BLOBS],
    bones: [Vec4; 2 * BONES],
}

impl ShadowMaterial {
    fn spawn(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<Self>>,
    ) {
        let mut quads = Quads::default();
        for index in 0..CASTERS {
            quads.push_with(Vec3::ZERO, [0.0; 3], [index as f32, 0.0, 0.0, 0.0]);
        }
        commands.spawn((
            Name::new("shadows of the locals"),
            Mesh3d(meshes.add(quads.into_mesh())),
            MeshMaterial3d(materials.add(Self {
                casters: Casters {
                    casters: [Caster::default(); CASTERS],
                },
            })),
            Transform::default(),
            Quads::unbounded(),
        ));
    }

    fn cast(
        camera: Single<&GlobalTransform, With<PlayerCamera>>,
        lamps: Res<Lamps>,
        silhouettes: Query<(Entity, &Silhouette)>,
        shadows: Query<&MeshMaterial3d<Self>>,
        mut materials: ResMut<Assets<Self>>,
        mut near: Local<Vec<(f32, Entity)>>,
    ) {
        let eye = camera.translation();
        near.clear();
        near.extend(
            silhouettes
                .iter()
                .map(|(entity, silhouette)| (silhouette.feet.distance(eye), entity))
                .filter(|(distance, _)| *distance < CAST_RANGE),
        );
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        for handle in &shadows {
            let Some(mut material) = materials.get_mut(handle) else {
                continue;
            };
            let casters = &mut material.casters.casters;
            *casters = [Caster::default(); CASTERS];
            let near = near
                .iter()
                .filter_map(|&(_, entity)| silhouettes.get(entity).ok());
            for (caster, (_, silhouette)) in casters.iter_mut().zip(near) {
                *caster = Caster::new(silhouette, &lamps.0);
            }
        }
    }

    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("shadows.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl Caster {
    fn new(silhouette: &Silhouette, lamps: &[Lamp]) -> Self {
        let feet = silhouette.feet;
        let mut total = Vec3::splat(AMBIENT);
        let mut strongest = [(0.0f32, None::<&Lamp>); LIGHTS - 1];
        for lamp in lamps {
            let lux = lamp.lux(feet);
            total += lamp.color * lux;
            if let Some(weakest) = strongest
                .iter_mut()
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .filter(|weakest| lux > weakest.0)
            {
                *weakest = (lux, Some(lamp));
            }
        }
        let mut caster = Self {
            ground: Vec4::splat(feet.y),
            ..default()
        };
        let mut low = feet.xz() - Vec2::splat(MARGIN);
        let mut high = feet.xz() + Vec2::splat(MARGIN);
        for (index, (lux, lamp)) in strongest.iter().enumerate() {
            let Some(lamp) = lamp else {
                continue;
            };
            caster.lights[index] = lamp.at.extend(LAMP_SIZE);
            caster.tints[index] = (lamp.color * *lux / total * DARKEST).extend(0.0);
            // The top of the head lands this far from the feet.
            let above = lamp.at.y - feet.y;
            let reach = if above > silhouette.height {
                (feet.xz() - lamp.at.xz()) * silhouette.height / (above - silhouette.height)
            } else {
                (feet.xz() - lamp.at.xz()).normalize_or_zero() * LONGEST
            };
            let tip = feet.xz() + reach.clamp_length_max(LONGEST);
            low = low.min(tip - Vec2::splat(MARGIN));
            high = high.max(tip + Vec2::splat(MARGIN));
        }
        caster.lights[LIGHTS - 1] = (feet + Vec3::Y * SKY_HEIGHT).extend(SKY_SIZE);
        caster.tints[LIGHTS - 1] = Vec4::new(SKY, SKY, SKY, 0.0);
        caster.bounds = Vec4::new(low.x, low.y, high.x, high.y);
        for (index, blob) in silhouette.blobs.iter().enumerate() {
            caster.blobs[4 * index..4 * index + 4].copy_from_slice(&blob.gpu());
        }
        for (index, bone) in silhouette.bones.iter().enumerate() {
            caster.bones[2 * index] = bone.from.extend(bone.radius);
            caster.bones[2 * index + 1] = bone.to.extend(0.0);
        }
        caster
    }
}

impl Material for ShadowMaterial {
    fn vertex_shader() -> ShaderRef {
        Self::shader()
    }

    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Multiply
    }
}
