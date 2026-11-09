use std::f32::consts::FRAC_PI_2;
use std::hash::Hash;

use bevy::prelude::*;
use ld60_game_parts::paper::{PAPER_REFLECTANCE, PAPER_ROUGHNESS, Wrinkles};
use ld60_game_player::focus::{Focus, FocusTarget, View};

/// Camera height above a sheet while reading it.
const READING_HEIGHT: f32 = 0.2;
/// Keeps a sheet off the mat under it.
const LIFT: f32 = 0.0004;

/// Reading a sheet lying face up, prompted with `prompt`.
pub(super) fn reading(prompt: &'static str) -> FocusTarget {
    FocusTarget {
        prompt,
        focus: Focus::Paper,
        view: View::Local(
            Transform::from_xyz(0.0, READING_HEIGHT, 0.001).looking_at(Vec3::ZERO, -Vec3::Z),
        ),
    }
}

/// Lays a sheet of `size` showing `image` face up on `entity`. `document`
/// shapes its wrinkles.
pub(super) fn lay(
    commands: &mut Commands,
    entity: Entity,
    (image, size, document): (Handle<Image>, Vec2, impl Hash),
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let wrinkles = Wrinkles::sticky(size, document);
    commands.entity(entity).with_child((
        Mesh3d(meshes.add(wrinkles.mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(image),
            perceptual_roughness: PAPER_ROUGHNESS,
            reflectance: PAPER_REFLECTANCE,
            double_sided: true,
            cull_mode: None,
            ..default()
        })),
        wrinkles,
        Transform::from_xyz(0.0, LIFT, 0.0).with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
    ));
}
