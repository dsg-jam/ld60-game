use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use ld60_core::layout::counter::CounterLayout;
use ld60_core::layout::desk::COMPUTER_SPOT;
use ld60_geometry::block::Block;

use super::COMPUTER_YAW;
use super::computer::ComputerLayout;
use crate::device::{Connected, Header};

/// Where the plug of the debug cable lies while it is out, see
/// [`CounterLayout::place`].
pub(super) const PLUG_SPOT: Vec2 = Vec2::new(0.36, -0.62);
pub(super) const PLUG_YAW: f32 = 0.6;
/// The cable leaves the plug at its +Z end.
pub(super) const PLUG: Vec3 = Vec3::new(0.013, 0.0055, 0.016);
/// The boot around the cable where it leaves the monitor, long along the
/// cable.
const BOOT: Vec3 = Vec3::new(0.007, 0.007, 0.014);
/// How high the port for the cable sits on the side of the monitor.
const PORT_HEIGHT: f32 = 0.025;
const RADIUS: f32 = 0.0016;
const SEGMENTS: usize = 32;
/// How far the cable sags below its curve, per meter between its ends.
const SAG: f32 = 0.12;
/// How far before the edge of the mat the cable starts to rise onto it, so
/// it drapes over the edge.
const MAT_MARGIN: f32 = 0.012;
/// How far the cable goes straight out of the boot and the plug before it
/// bends, at most.
const STRAIGHT: f32 = 0.035;
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
    /// Where it leaves the boot on the monitor.
    socket: Vec3,
    /// Which way it leaves the monitor.
    out: Vec3,
    /// The counter top, which the cable lies on.
    floor: f32,
    /// The mat on the counter, which the cable lies on where it crosses it.
    mat: Block,
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
        // A port low on the side of the monitor that faces the mat.
        let computer = counter.place(COMPUTER_SPOT, COMPUTER_YAW);
        let monitor = ComputerLayout::new().monitor_body();
        let port = computer.transform_point(Vec3::new(
            monitor.min().x,
            monitor.min().y + PORT_HEIGHT,
            monitor.center.z,
        ));
        let out = computer.rotation * Vec3::NEG_X;
        let plug_black = materials.add(PLUG_BLACK);
        commands.spawn((
            Name::new("debug cable boot"),
            Mesh3d(meshes.add(Cuboid::from_size(BOOT))),
            MeshMaterial3d(plug_black.clone()),
            Transform::from_translation(port + out * (BOOT.z / 2.0 - 0.002))
                .looking_to(out, Vec3::Y),
            Pickable::IGNORE,
        ));
        commands.spawn((
            Name::new("debug cable plug"),
            Plug { rest },
            rest,
            Mesh3d(meshes.add(Cuboid::from_size(PLUG))),
            MeshMaterial3d(plug_black),
            Pickable::IGNORE,
        ));
        // Each piece is a little longer than its stretch, so the bends close.
        let mesh = meshes.add(Cylinder::new(0.5, 1.0));
        let material = materials.add(CABLE_GRAY);
        commands
            .spawn((
                Name::new("debug cable"),
                Cable {
                    socket: port + out * (BOOT.z - 0.002),
                    out,
                    floor: counter.place(PLUG_SPOT, 0.0).translation.y + RADIUS,
                    mat: counter.mat,
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
            // The plug stands straight up off the board, so the cable leaves
            // the device and does not run along it.
            Some((header, _)) => {
                let (_, rotation, center) = header.to_scale_rotation_translation();
                let away = rotation * Vec3::NEG_Y;
                Transform::from_translation(center + away * PLUG.z / 2.0)
                    .with_rotation(rotation * Quat::from_rotation_x(FRAC_PI_2))
            }
            None => rest.rest,
        };
        let exit = transform.rotation * Vec3::Z;
        let end = transform.translation + exit * (PLUG.z / 2.0);
        let points = cable.points(end, exit);
        for (segment, mut transform) in &mut segments {
            let (from, to) = (points[segment.0], points[segment.0 + 1]);
            let along = to - from;
            *transform = Transform::from_translation((from + to) / 2.0)
                .with_rotation(Quat::from_rotation_arc(
                    Vec3::Y,
                    along.normalize_or(Vec3::Y),
                ))
                .with_scale(Vec3::new(
                    2.0 * RADIUS,
                    along.length() + RADIUS,
                    2.0 * RADIUS,
                ));
        }
    }

    /// Points along the cable from the computer to `end`, which it reaches
    /// going against `exit`. It leaves both ends straight, sags in between,
    /// and stays on the counter and the mat.
    fn points(&self, end: Vec3, exit: Vec3) -> [Vec3; SEGMENTS + 1] {
        let span = self.socket.distance(end);
        let straight = STRAIGHT.min(span / 3.0);
        let controls = [
            self.socket,
            self.socket + self.out * straight,
            end + exit * straight,
            end,
        ];
        let sag = SAG * span;
        std::array::from_fn(|index| {
            let t = index as f32 / SEGMENTS as f32;
            let s = 1.0 - t;
            let curve = controls[0] * s * s * s
                + controls[1] * 3.0 * s * s * t
                + controls[2] * 3.0 * s * t * t
                + controls[3] * t * t * t;
            // Flat at both ends, so the cable leaves them straight.
            let point = curve - Vec3::Y * sag * 16.0 * t * t * s * s;
            point.with_y(point.y.max(self.floor_at(point)))
        })
    }

    /// The lowest the middle of the cable gets at `point`: on the mat where
    /// it crosses it, otherwise on the counter.
    fn floor_at(&self, point: Vec3) -> f32 {
        let (min, max) = (self.mat.min(), self.mat.max());
        let outside = Vec2::new(
            (min.x - point.x).max(point.x - max.x),
            (min.z - point.z).max(point.z - max.z),
        )
        .max_element();
        let rise = (1.0 - outside / MAT_MARGIN).clamp(0.0, 1.0);
        self.floor + (max.y + RADIUS - self.floor) * rise
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cable() -> Cable {
        Cable {
            socket: Vec3::new(0.0, 1.05, 0.0),
            out: Vec3::NEG_X,
            floor: 1.0,
            mat: Block::new(Vec3::new(0.2, 0.004, 0.2)).translated(Vec3::new(-0.3, 1.0, 0.0)),
        }
    }

    #[test]
    fn the_cable_runs_between_its_ends_above_the_counter() {
        let cable = cable();
        let end = Vec3::new(-0.4, 1.2, 0.1);
        let points = cable.points(end, Vec3::Y);
        assert_eq!(points[0], cable.socket);
        assert!(points[SEGMENTS].distance(end) < 1e-6);
        assert!(points.iter().all(|point| point.y >= cable.floor_at(*point)));
        assert!(cable.floor_at(Vec3::new(-0.3, 0.0, 0.0)) > cable.mat.max().y);
    }

    #[test]
    fn the_cable_leaves_both_ends_straight() {
        let cable = cable();
        let end = Vec3::new(-0.4, 1.2, 0.1);
        let points = cable.points(end, Vec3::Y);
        let first = (points[1] - points[0]).normalize();
        let last = (points[SEGMENTS - 1] - points[SEGMENTS]).normalize();
        assert!(first.dot(cable.out) > 0.7, "{first}");
        assert!(last.dot(Vec3::Y) > 0.7, "{last}");
    }
}
