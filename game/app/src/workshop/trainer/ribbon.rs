use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use ld60_game_core::layout::counter::CounterLayout;

use super::Trainer;
use super::layout::CASE;
use super::movement::CABLE_PLUG;

/// Width of the flat cable and how many pieces it bends in.
const WIDTH: f32 = 0.012;
const PIECES: usize = 32;
/// The cable leaves each end straight for this share of the way between
/// them before it bends.
const STRAIGHT: f32 = 0.35;
/// It lies this far above the mat.
const LIFT: f32 = 0.0008;
/// Ends closer than this to where they were need no new bend.
const MOVED: f32 = 0.0002;

const GREY: Color = Color::srgb_u8(0xb8, 0xb6, 0xae);

/// The flat cable from a panel to the movement it works on.
#[derive(Component, Debug)]
pub(super) struct Ribbon {
    panel: Entity,
    ends: [Vec3; 2],
}

impl Ribbon {
    /// Lays a cable from each panel to its movement, follows the movement
    /// when it is picked up, and takes it away with the movement.
    pub(super) fn lay(
        trainers: Query<(Entity, &Trainer, &GlobalTransform)>,
        models: Query<&GlobalTransform>,
        mut ribbons: Query<(Entity, &mut Ribbon, &Mesh3d)>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut commands: Commands,
    ) {
        for (panel, trainer, at) in &trainers {
            let ribbon = ribbons
                .iter_mut()
                .find(|(_, ribbon, _)| ribbon.panel == panel);
            let model = trainer.model.and_then(|model| models.get(model).ok());
            let Some(model) = model else {
                if let Some((entity, ..)) = ribbon {
                    commands.entity(entity).despawn();
                }
                continue;
            };
            let socket = at.transform_point(Vec3::new(0.0, 0.006, CASE.z / 2.0));
            let out = at.rotation() * Vec3::Z;
            let plug = model.transform_point(CABLE_PLUG);
            let back = model.rotation() * Vec3::NEG_Z;
            let ends = [socket, plug];
            match ribbon {
                Some((_, mut ribbon, mesh)) => {
                    let moved = ribbon
                        .ends
                        .iter()
                        .zip(ends)
                        .any(|(was, now)| was.distance(now) > MOVED);
                    if moved && let Some(mut mesh) = meshes.get_mut(&mesh.0) {
                        ribbon.ends = ends;
                        *mesh = Self::mesh(socket, out, plug, back);
                    }
                }
                None => {
                    commands.spawn((
                        Name::new("panel cable"),
                        Ribbon { panel, ends },
                        Mesh3d(meshes.add(Self::mesh(socket, out, plug, back))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: GREY,
                            perceptual_roughness: 0.7,
                            double_sided: true,
                            cull_mode: None,
                            ..default()
                        })),
                        Transform::default(),
                    ));
                }
            }
        }
    }

    /// A flat strip from `from`, leaving along `out`, to `to`, arriving
    /// against `back`. It sags onto the mat between.
    fn mesh(from: Vec3, out: Vec3, to: Vec3, back: Vec3) -> Mesh {
        let floor = CounterLayout::new().mat.max().y + LIFT;
        let reach = from.distance(to) * STRAIGHT;
        let sag = |point: Vec3| point.with_y(floor);
        let controls = [from, sag(from + out * reach), sag(to + back * reach), to];
        let point = |t: f32| {
            let u = 1.0 - t;
            let [a, b, c, d] = controls;
            let at = a * u * u * u + b * 3.0 * u * u * t + c * 3.0 * u * t * t + d * t * t * t;
            at.with_y(at.y.max(floor))
        };
        let mut positions = Vec::with_capacity(2 * (PIECES + 1));
        let mut normals = Vec::with_capacity(2 * (PIECES + 1));
        for index in 0..=PIECES {
            let t = index as f32 / PIECES as f32;
            let here = point(t);
            let ahead = point((t + 0.01).min(1.0)) - point((t - 0.01).max(0.0));
            let side = ahead.cross(Vec3::Y).normalize_or(Vec3::X) * WIDTH / 2.0;
            let normal = side.cross(ahead).normalize_or(Vec3::Y);
            positions.extend([(here - side).to_array(), (here + side).to_array()]);
            normals.extend([normal.to_array(), normal.to_array()]);
        }
        let indices = (0..PIECES as u32)
            .flat_map(|piece| {
                let at = piece * 2;
                [at, at + 2, at + 1, at + 1, at + 2, at + 3]
            })
            .collect();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
    }
}
