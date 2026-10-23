use bevy::{asset::AssetEvent, light::NotShadowCaster, prelude::*};

use super::leaf::{blank_print, reprinted, reversed};
use super::sheet::PaperMaterial;

pub(super) struct BackPlugin;

impl Plugin for BackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, BlankBack::follow_front)
            .add_observer(BlankBack::spawn);
    }
}

/// Gives a sheet a back of blank paper, so it shows from behind. The back
/// is the sheet's mesh turned inside out, so it is lit like the front.
#[derive(Component, Debug, Default)]
pub struct BlankBack {
    back: Option<Entity>,
}

impl BlankBack {
    /// The front only has its paper once it is spawned.
    fn spawn(
        add: On<Add<MeshMaterial3d<PaperMaterial>>>,
        mut fronts: Query<(&mut BlankBack, &Mesh3d, &MeshMaterial3d<PaperMaterial>)>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<PaperMaterial>>,
        mut images: ResMut<Assets<Image>>,
        mut commands: Commands,
    ) {
        let Ok((mut blank, mesh, material)) = fronts.get_mut(add.entity) else {
            return;
        };
        let (Some(turned), Some(material)) =
            (meshes.get(mesh).map(reversed), materials.get(material))
        else {
            return;
        };
        let mesh = meshes.add(turned);
        let mut paper = reprinted(material, blank_print(&mut images));
        paper.base.emissive = LinearRgba::BLACK;
        let paper = materials.add(paper);
        if let Some(old) = blank.back.take() {
            commands.entity(old).despawn();
        }
        blank.back = Some(
            commands
                .spawn((
                    Name::new("back of sheet"),
                    Mesh3d(mesh),
                    MeshMaterial3d(paper),
                    NotShadowCaster,
                    ChildOf(add.entity),
                ))
                .id(),
        );
    }

    /// Bends the back along with the front.
    fn follow_front(
        mut events: MessageReader<AssetEvent<Mesh>>,
        fronts: Query<(&BlankBack, &Mesh3d)>,
        backs: Query<&Mesh3d>,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        for event in events.read() {
            let AssetEvent::Modified { id } = event else {
                continue;
            };
            for (blank, front) in &fronts {
                if front.id() != *id {
                    continue;
                }
                let Some(back) = blank.back.and_then(|back| backs.get(back).ok()) else {
                    continue;
                };
                let Some(turned) = meshes.get(front).map(reversed) else {
                    continue;
                };
                if let Some(mut mesh) = meshes.get_mut(back) {
                    *mesh = turned;
                }
            }
        }
    }
}
