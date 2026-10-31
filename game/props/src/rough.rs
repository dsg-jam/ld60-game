//! Props seen from afar, as a few plain boxes and cylinders. All share two
//! meshes and one material per color and finish, so a street of shelves
//! costs little.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::surface::{Finish, SurfaceMaterial};

/// Colors that differ by less than one step of this many per channel share
/// a material.
const COLOR_STEPS: f32 = 8.0;

pub(crate) struct RoughPlugin;

impl Plugin for RoughPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoughKit>().add_observer(Rough::on_add);
    }
}

/// A plain part of a prop seen from afar. Its transform scales a unit mesh
/// to its size.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Rough {
    round: bool,
    color: Color,
    finish: Finish,
}

/// The meshes of rough parts, and their materials made so far.
#[derive(Resource)]
struct RoughKit {
    cube: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    materials: HashMap<([u8; 3], u32), Handle<SurfaceMaterial>>,
}

impl FromWorld for RoughKit {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        Self {
            cube: meshes.add(Cuboid::from_length(1.0)),
            cylinder: meshes.add(Cylinder::new(0.5, 1.0)),
            materials: HashMap::default(),
        }
    }
}

impl Rough {
    /// A box of `color` with `finish` filling `block`.
    pub(crate) fn cuboid(
        parent: &mut ChildSpawnerCommands,
        block: Block,
        color: Color,
        finish: Finish,
    ) {
        Self::spawn(parent, false, block, color, finish);
    }

    /// An upright cylinder filling `block`.
    pub(crate) fn cylinder(
        parent: &mut ChildSpawnerCommands,
        block: Block,
        color: Color,
        finish: Finish,
    ) {
        Self::spawn(parent, true, block, color, finish);
    }

    fn spawn(
        parent: &mut ChildSpawnerCommands,
        round: bool,
        block: Block,
        color: Color,
        finish: Finish,
    ) {
        parent.spawn((
            Self {
                round,
                color,
                finish,
            },
            block.transform().with_scale(block.size),
        ));
    }

    fn on_add(
        add: On<Add<Rough>>,
        roughs: Query<&Rough>,
        mut kit: ResMut<RoughKit>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
        mut commands: Commands,
    ) {
        let Ok(&rough) = roughs.get(add.entity) else {
            return;
        };
        let srgba = rough.color.to_srgba();
        let step = |channel: f32| (channel * COLOR_STEPS).round() as u8;
        let key = (
            [step(srgba.red), step(srgba.green), step(srgba.blue)],
            rough.finish as u32,
        );
        let material = kit
            .materials
            .entry(key)
            .or_insert_with(|| {
                let [r, g, b] = key.0.map(|channel| f32::from(channel) / COLOR_STEPS);
                surfaces.add(rough.finish.scaled(Color::srgb(r, g, b)))
            })
            .clone();
        let mesh = if rough.round {
            kit.cylinder.clone()
        } else {
            kit.cube.clone()
        };
        commands
            .entity(add.entity)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
}
