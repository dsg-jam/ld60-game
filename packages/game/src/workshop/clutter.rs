mod props;
mod screwdriver;
mod sides;

use bevy::prelude::*;
use ld60_core::layout::counter::CounterLayout;
use ld60_geometry::surface::SurfaceMaterial;

use self::props::{Finishes, Props};
use self::screwdriver::Screwdriver;
#[cfg(test)]
use super::footprint::Footprint;

const MUG_SPOT: Vec2 = Vec2::new(-0.62, 0.45);
const SCREWDRIVER_SPOT: Vec2 = Vec2::new(-0.38, 0.62);
const SCREWDRIVER_YAW: f32 = 0.4;

pub(crate) struct ClutterPlugin;

impl Plugin for ClutterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Screwdriver::follow)
            .add_observer(Clutter::on_add);
    }
}

/// Small things lying around on the counter.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Clutter;

#[cfg(test)]
impl Clutter {
    pub(super) fn footprints(counter: &CounterLayout) -> Vec<Footprint> {
        let mut footprints = vec![
            Footprint::new("mug", counter.place(MUG_SPOT, 0.0), Props::mug_area()),
            Footprint::new(
                "screwdriver",
                counter.place(SCREWDRIVER_SPOT, SCREWDRIVER_YAW),
                Props::screwdriver_area(),
            ),
        ];
        footprints.extend(sides::footprints(counter));
        footprints
    }
}

impl Clutter {
    fn on_add(
        add: On<Add<Clutter>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let counter = CounterLayout::new();
        let finishes = Finishes::new(&mut surfaces);
        let mut props = Props {
            meshes: &mut meshes,
            finishes: &finishes,
        };
        commands.entity(add.entity).with_children(|clutter| {
            clutter
                .spawn((counter.place(MUG_SPOT, 0.0), Visibility::default()))
                .with_children(|mug| props.mug(mug));
            let screwdriver = counter.place(SCREWDRIVER_SPOT, SCREWDRIVER_YAW);
            clutter
                .spawn((
                    Name::new("screwdriver"),
                    Screwdriver::new(screwdriver),
                    screwdriver,
                    Visibility::default(),
                ))
                .with_children(|screwdriver| props.screwdriver(screwdriver));
            sides::spawn(clutter, props.meshes, &mut materials);
        });
    }
}
