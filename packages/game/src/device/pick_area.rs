use bevy::{light::NotShadowCaster, prelude::*};

/// An invisible box of this size around a small part of a device. It
/// catches the pointer for the part, so the part is easy to click.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(super) struct PickArea(pub(super) Vec3);

impl PickArea {
    pub(super) fn on_add(
        add: On<Add<PickArea>>,
        areas: Query<&PickArea>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut material: Local<Option<Handle<StandardMaterial>>>,
        mut commands: Commands,
    ) {
        let Ok(area) = areas.get(add.entity) else {
            return;
        };
        let material = material
            .get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::NONE,
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })
            })
            .clone();
        commands.entity(add.entity).insert((
            Mesh3d(meshes.add(Cuboid::from_size(area.0))),
            MeshMaterial3d(material),
            NotShadowCaster,
        ));
    }
}
