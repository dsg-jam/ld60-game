use std::f32::consts::PI;

use bevy::prelude::*;

use super::{PAPER, PAPER_REFLECTANCE, PAPER_SHADE, Paper};
use crate::player::focus::Focused;

/// Seconds a page takes to turn over.
const TURN_TIME: f32 = 0.3;
/// Strips a turning page bends along.
const STRIPS: usize = 4;
/// How far a turning page bends at most, in radians over its width.
const CURL: f32 = 0.9;
/// Lifts a turning page off the sheet below, so they don't flicker.
const LIFT: f32 = 0.0004;

/// How the pages of a [`Paper`] are held together, which decides how they
/// turn over.
#[derive(Component, Clone, Debug)]
pub(crate) struct Binding {
    /// A point on the line the pages turn about, in the sheet's space.
    hinge: Vec3,
    /// Points from the hinge across the page.
    across: Vec3,
    /// From the hinge to the near edge of the page.
    gap: f32,
    /// The page last shown.
    shown: usize,
    strip: Option<Handle<Mesh>>,
}

impl Binding {
    /// Rings `gap` left of the sheet of `size`, so the pages turn over to the
    /// left.
    pub(crate) fn rings(size: Vec2, gap: f32) -> Self {
        Self::new(Vec3::NEG_X * (size.x / 2.0 + gap), Vec3::X, gap)
    }

    /// A clip on the top edge of the sheet of `size`, so the pages turn over
    /// the top.
    pub(crate) fn clip(size: Vec2) -> Self {
        Self::new(Vec3::Y * size.y / 2.0, Vec3::NEG_Y, 0.0)
    }

    fn new(hinge: Vec3, across: Vec3, gap: f32) -> Self {
        Self {
            hinge,
            across,
            gap,
            shown: 0,
            strip: None,
        }
    }

    /// The axis the page turns up about.
    fn axis(&self) -> Vec3 {
        self.across.cross(Vec3::Z)
    }

    /// Turns a loose page over the binding when the focused paper changes
    /// page. It turns to the left or over the top going forward and back
    /// going back.
    pub(super) fn turn_pages(
        mut papers: Query<(Entity, &Paper, &mut Binding), Changed<Paper>>,
        sheets: Query<(&Transform, &ChildOf)>,
        focused: Res<Focused>,
        look: Res<LeafLook>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut commands: Commands,
    ) {
        for (entity, paper, mut binding) in &mut papers {
            let current = paper.current_page();
            if binding.shown == current {
                continue;
            }
            let forward = current > binding.shown;
            binding.shown = current;
            let Ok((sheet, parent)) = sheets.get(paper.sheet) else {
                continue;
            };
            if focused.get() != Some(entity) {
                continue;
            }
            let length = (paper.size * binding.across.truncate().abs()).element_sum();
            let width = (paper.size * binding.axis().truncate().abs()).element_sum();
            let strip = length / STRIPS as f32;
            let size =
                binding.across.truncate().abs() * strip + binding.axis().truncate().abs() * width;
            let mesh = binding
                .strip
                .get_or_insert_with(|| meshes.add(Rectangle::from_size(size)))
                .clone();
            let root =
                sheet.mul_transform(Transform::from_translation(binding.hinge + Vec3::Z * LIFT));
            let mut leaf = Leaf {
                elapsed: 0.0,
                forward,
                axis: binding.axis(),
                joints: Vec::with_capacity(STRIPS + 1),
            };
            let mut parent_joint = commands
                .spawn((root, Visibility::Inherited, ChildOf(parent.parent())))
                .id();
            let mut offset = 0.0;
            for (index, rotation) in leaf.rotations().enumerate() {
                let joint = commands
                    .spawn((
                        Transform::from_translation(binding.across * offset)
                            .with_rotation(rotation),
                        Visibility::Inherited,
                        ChildOf(parent_joint),
                    ))
                    .id();
                if index > 0 {
                    commands.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(look.0.clone()),
                        Transform::from_translation(binding.across * strip / 2.0),
                        Pickable::IGNORE,
                        ChildOf(joint),
                    ));
                }
                leaf.joints.push(joint);
                parent_joint = joint;
                offset = if index == 0 { binding.gap } else { strip };
            }
            commands.entity(leaf.joints[0]).insert(leaf);
        }
    }
}

/// The look of a turning page, plain paper on both sides.
#[derive(Resource, Debug)]
pub(super) struct LeafLook(Handle<StandardMaterial>);

impl FromWorld for LeafLook {
    fn from_world(world: &mut World) -> Self {
        let paper = PAPER.to_linear() * PAPER_SHADE.to_linear().red;
        Self(
            world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: paper.into(),
                    perceptual_roughness: 0.95,
                    reflectance: PAPER_REFLECTANCE,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                }),
        )
    }
}

/// A page turning over its [`Binding`]. The first joint turns it about the
/// binding and the others bend it.
#[derive(Component, Debug)]
pub(super) struct Leaf {
    elapsed: f32,
    forward: bool,
    axis: Vec3,
    joints: Vec<Entity>,
}

impl Leaf {
    /// The rotation of each joint, first the turn and then the bends.
    fn rotations(&self) -> impl Iterator<Item = Quat> + use<> {
        let t = (self.elapsed / TURN_TIME).min(1.0);
        let eased = t * t * (3.0 - 2.0 * t);
        let (turn, lag) = if self.forward {
            (eased * PI, -1.0)
        } else {
            ((1.0 - eased) * PI, 1.0)
        };
        let bend = lag * CURL * (t * PI).sin() / STRIPS as f32;
        let axis = self.axis;
        (0..=STRIPS)
            .map(move |index| Quat::from_axis_angle(axis, if index == 0 { turn } else { bend }))
    }

    pub(super) fn animate(
        time: Res<Time>,
        mut leaves: Query<(&mut Leaf, &ChildOf)>,
        mut transforms: Query<&mut Transform>,
        mut commands: Commands,
    ) {
        for (mut leaf, parent) in &mut leaves {
            leaf.elapsed += time.delta_secs();
            if leaf.elapsed >= TURN_TIME {
                commands.entity(parent.parent()).despawn();
                continue;
            }
            for (&joint, rotation) in leaf.joints.iter().zip(leaf.rotations()) {
                if let Ok(mut transform) = transforms.get_mut(joint) {
                    transform.rotation = rotation;
                }
            }
        }
    }
}
