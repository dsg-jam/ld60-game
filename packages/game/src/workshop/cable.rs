use bevy::prelude::*;
use ld60_core::layout::counter::CounterLayout;

use crate::device::{Connected, Header};

/// Where the plug of the debug cable lies while it is out, see
/// [`CounterLayout::place`].
pub(super) const PLUG_SPOT: Vec2 = Vec2::new(0.36, -0.62);
pub(super) const PLUG_YAW: f32 = 0.6;
pub(super) const PLUG: Vec3 = Vec3::new(0.012, 0.006, 0.02);
/// Where the cable comes out from under the computer.
const SOCKET_SPOT: Vec2 = Vec2::new(0.56, -0.8);
const RADIUS: f32 = 0.0015;
const SEGMENTS: usize = 16;
/// How far the cable sags below a straight line, per meter between its ends.
const SAG: f32 = 0.15;
const PLUG_BLACK: Color = Color::srgb_u8(0x22, 0x22, 0x24);
const CABLE_GRAY: Color = Color::srgb_u8(0x3a, 0x3c, 0x40);

pub(super) struct CablePlugin;

impl Plugin for CablePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Cable::spawn)
            .add_systems(Update, Cable::follow);
    }
}

/// The debug cable from the computer to a device, or to its plug on the
/// counter.
#[derive(Component, Debug)]
struct Cable {
    /// Where it leaves the computer.
    socket: Vec3,
    /// The counter top, which the cable lies on.
    floor: f32,
}

/// The end of the cable that goes onto the [`Header`] of a device.
#[derive(Component, Debug)]
struct Plug {
    rest: Transform,
}

/// One straight piece of the [`Cable`], counted from the computer.
#[derive(Component, Debug)]
struct Segment(usize);

impl Cable {
    fn spawn(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let counter = CounterLayout::new();
        let rest = counter.place(PLUG_SPOT, PLUG_YAW);
        let rest = rest.with_translation(rest.translation + Vec3::Y * PLUG.y / 2.0);
        let socket = counter.place(SOCKET_SPOT, 0.0).translation + Vec3::Y * RADIUS;
        commands.spawn((
            Name::new("debug cable plug"),
            Plug { rest },
            rest,
            Mesh3d(meshes.add(Cuboid::from_size(PLUG))),
            MeshMaterial3d(materials.add(PLUG_BLACK)),
            Pickable::IGNORE,
        ));
        let mesh = meshes.add(Cuboid::from_length(1.0));
        let material = materials.add(CABLE_GRAY);
        commands
            .spawn((
                Name::new("debug cable"),
                Cable {
                    socket,
                    floor: socket.y,
                },
                Transform::IDENTITY,
                Visibility::Inherited,
            ))
            .with_children(|cable| {
                for index in 0..SEGMENTS {
                    cable.spawn((
                        Segment(index),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::default(),
                        Pickable::IGNORE,
                    ));
                }
            });
    }

    /// Puts the plug on the header of the plugged in device, or back on the
    /// counter, and hangs the cable between it and the computer.
    fn follow(
        headers: Query<(&GlobalTransform, &ChildOf), With<Header>>,
        plugged: Query<(), With<Connected>>,
        cable: Single<&Cable>,
        mut plug: Single<(&Plug, &mut Transform), Without<Segment>>,
        mut segments: Query<(&Segment, &mut Transform), Without<Plug>>,
    ) {
        let header = headers
            .iter()
            .find(|(_, parent)| plugged.contains(parent.parent()));
        let (rest, transform) = &mut *plug;
        **transform = match header {
            Some((header, _)) => {
                let (_, rotation, center) = header.to_scale_rotation_translation();
                let down = rotation * Vec3::NEG_Y;
                Transform::from_translation(center + down * PLUG.y).with_rotation(rotation)
            }
            None => rest.rest,
        };
        let points = cable.points(transform.translation);
        for (segment, mut transform) in &mut segments {
            let (from, to) = (points[segment.0], points[segment.0 + 1]);
            let along = to - from;
            *transform = Transform::from_translation((from + to) / 2.0)
                .with_rotation(Quat::from_rotation_arc(
                    Vec3::Z,
                    along.normalize_or(Vec3::Z),
                ))
                .with_scale(Vec3::new(2.0 * RADIUS, 2.0 * RADIUS, along.length()));
        }
    }

    /// Points along the cable from the computer to `end`, sagging but never
    /// below the counter top.
    fn points(&self, end: Vec3) -> [Vec3; SEGMENTS + 1] {
        let sag = SAG * self.socket.distance(end);
        std::array::from_fn(|index| {
            let t = index as f32 / SEGMENTS as f32;
            let point = self.socket.lerp(end, t) - Vec3::Y * sag * 4.0 * t * (1.0 - t);
            point.with_y(point.y.max(self.floor))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cable_runs_between_its_ends_above_the_counter() {
        let cable = Cable {
            socket: Vec3::new(0.0, 1.0, 0.0),
            floor: 1.0,
        };
        let end = Vec3::new(0.4, 1.2, 0.1);
        let points = cable.points(end);
        assert_eq!(points[0], cable.socket);
        assert!(points[SEGMENTS].distance(end) < 1e-6);
        assert!(points.iter().all(|point| point.y >= cable.floor));
        assert!(points[SEGMENTS / 2].y < (cable.socket.y + end.y) / 2.0);
    }
}
