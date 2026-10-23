use std::f32::consts::FRAC_PI_2;

use bevy::{math::Affine3A, prelude::*};
use ld60_core::layout::counter::CounterLayout;
use ld60_core::layout::desk::COMPUTER_SPOT;
use ld60_geometry::block::Block;

use super::COMPUTER_YAW;
use super::computer::ComputerLayout;
use crate::device::{Casing, Connected, Header};

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
/// The cable is this much longer than the way between its ends, so it
/// hangs a little, and comes out of the computer or goes back in to keep it
/// so.
const SLACK: f32 = 1.25;
const EXTRA: f32 = 0.05;
/// How fast the cable comes out of or goes back into the computer, in
/// meters per second.
const REEL: f32 = 0.6;
/// The cable is a chain of this many points.
const POINTS: usize = 48;
/// Steps of the simulation per frame. Small steps keep it stiff and calm.
const SUBSTEPS: usize = 12;
const FRAME: f32 = 1.0 / 60.0;
/// How soft the cable bends, in meters per newton. Zero would be a rod.
const BEND_COMPLIANCE: f32 = 2e-7;
/// How much of its speed the cable loses per second.
const DAMPING: f32 = 8.0;
/// How much of its sliding a point on the counter keeps per step.
const FRICTION: f32 = 0.3;
/// A point on the counter slower than this, in meters per second, stops.
const STICK: f32 = 0.05;
/// The cable sleeps once no point moves farther than this in a frame.
const SLEEP: f32 = 5e-5;
/// How far before the edge of the mat the cable starts to rise onto it, so
/// it drapes over the edge.
const MAT_MARGIN: f32 = 0.006;
const GRAVITY: f32 = 9.81;
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
///
/// It is a chain of points that keep their distance and resist bending, as
/// in Macklin et al., "XPBD" (2016) and "Small Steps in Physics Simulation"
/// (2019). The points stay out of the device, the computer and the counter.
/// The boot and the plug hold both ends straight.
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
    /// Boxes the cable stays out of, with where they are.
    solids: Vec<(Affine3A, Block)>,
    points: Vec<Vec3>,
    /// Where the points were a step ago, which gives their speed.
    previous: Vec<Vec3>,
    /// How long the cable is now, see [`SLACK`].
    length: f32,
    /// Where the plug end was when the cable went to sleep.
    asleep: Option<Vec3>,
    /// The ends a frame ago. Each step moves them a little of the way to
    /// where they are now, so a fast turn of the device does not yank the
    /// cable or sweep through it.
    last: Option<Ends>,
}

/// The end of the cable that goes onto the [`Header`] of a device.
#[derive(Component, Debug)]
struct Plug {
    rest: Transform,
}

/// One straight piece of the [`Cable`], counted from the computer.
#[derive(Component, Debug)]
struct Segment(usize);

/// Where the cable ends at the plug, the way it leaves the plug, and the
/// box of the device it is plugged into.
#[derive(Clone, Copy, Debug)]
struct Ends {
    end: Vec3,
    exit: Vec3,
    device: Option<(Affine3A, Block)>,
}

impl Ends {
    /// The ends `t` of the way from these to `other`.
    fn towards(&self, other: &Self, t: f32) -> Self {
        let device = match (self.device, other.device) {
            (Some((from, _)), Some((to, bounds))) => {
                let (_, a, p) = from.to_scale_rotation_translation();
                let (_, b, q) = to.to_scale_rotation_translation();
                Some((
                    Affine3A::from_rotation_translation(a.slerp(b, t), p.lerp(q, t)),
                    bounds,
                ))
            }
            (_, device) => device,
        };
        Self {
            end: self.end.lerp(other.end, t),
            exit: self.exit.lerp(other.exit, t).normalize_or(other.exit),
            device,
        }
    }
}

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
        let layout = ComputerLayout::new();
        let monitor = layout.monitor_body();
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
        let at_computer = computer.compute_affine();
        let mut cable = Self {
            socket: port + out * (BOOT.z - 0.002),
            out,
            floor: counter.place(PLUG_SPOT, 0.0).translation.y + RADIUS,
            mat: counter.mat,
            solids: vec![
                (at_computer, monitor),
                (at_computer, layout.keyboard_placement),
            ],
            points: Vec::new(),
            previous: Vec::new(),
            length: 0.0,
            asleep: None,
            last: None,
        };
        let exit = rest.rotation * Vec3::Z;
        cable.lay(rest.translation + exit * (PLUG.z / 2.0));
        // Each piece is a little longer than its stretch, so the bends close.
        let mesh = meshes.add(Cylinder::new(0.5, 1.0));
        let material = materials.add(CABLE_GRAY);
        commands
            .spawn((
                Name::new("debug cable"),
                cable,
                Transform::IDENTITY,
                Visibility::Inherited,
            ))
            .with_children(|cable| {
                for index in 0..POINTS - 1 {
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
    /// counter, and lets the cable move to it.
    fn follow(
        headers: Query<(&GlobalTransform, &ChildOf), With<Header>>,
        plugged: Query<(&GlobalTransform, &Casing), With<Connected>>,
        mut cable: Single<&mut Cable>,
        mut plug: Single<(&Plug, &mut Transform), Without<Segment>>,
        mut segments: Query<(&Segment, &mut Transform), Without<Plug>>,
    ) {
        let header = headers
            .iter()
            .find(|(_, parent)| plugged.contains(parent.parent()));
        let (rest, transform) = &mut *plug;
        **transform = match header {
            // The plug stands straight up off the board, so the cable leaves
            // the device and not along it.
            Some((header, _)) => {
                let (_, rotation, center) = header.to_scale_rotation_translation();
                let away = rotation * Vec3::NEG_Y;
                Transform::from_translation(center + away * PLUG.z / 2.0)
                    .with_rotation(rotation * Quat::from_rotation_x(FRAC_PI_2))
            }
            None => rest.rest,
        };
        let exit = transform.rotation * Vec3::Z;
        let ends = Ends {
            end: transform.translation + exit * (PLUG.z / 2.0),
            exit,
            device: header
                .and_then(|(_, parent)| plugged.get(parent.parent()).ok())
                .map(|(device, casing)| (device.affine(), *casing.bounds())),
        };
        if !cable.frame(&ends) {
            return;
        }
        for (segment, mut transform) in &mut segments {
            let (from, to) = (cable.points[segment.0], cable.points[segment.0 + 1]);
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

    /// Lays the cable out straight from the computer to `end` to fall into
    /// place from.
    fn lay(&mut self, end: Vec3) {
        self.length = Self::length_for(self.socket, end);
        self.points = (0..POINTS)
            .map(|index| {
                let t = index as f32 / (POINTS - 1) as f32;
                let point = self.socket.lerp(end, t);
                point.with_y(point.y.max(self.floor_at(point)))
            })
            .collect();
        self.previous = self.points.clone();
    }

    /// Moves the cable on by one frame, unless it sleeps and the plug has
    /// not moved. Returns whether it moved.
    fn frame(&mut self, ends: &Ends) -> bool {
        let wanted = Self::length_for(self.socket, ends.end);
        if wanted == self.length
            && self
                .asleep
                .is_some_and(|end| end.distance(ends.end) < SLEEP)
        {
            return false;
        }
        self.length += (wanted - self.length).clamp(-REEL * FRAME, REEL * FRAME);
        let before = self.points.clone();
        let from = self.last.unwrap_or(*ends);
        for substep in 1..=SUBSTEPS {
            let between = from.towards(ends, substep as f32 / SUBSTEPS as f32);
            self.step(FRAME / SUBSTEPS as f32, &between);
        }
        self.last = Some(*ends);
        let moved = before
            .iter()
            .zip(&self.points)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0, f32::max);
        self.asleep = (moved < SLEEP).then_some(ends.end);
        true
    }

    /// How long the cable wants to be between `socket` and `end`.
    fn length_for(socket: Vec3, end: Vec3) -> f32 {
        socket.distance(end) * SLACK + EXTRA
    }

    /// One small step of `dt` seconds.
    fn step(&mut self, dt: f32, ends: &Ends) {
        let last = POINTS - 1;
        let spacing = self.length / last as f32;
        let free = 3..last - 2;
        let keep = (1.0 - DAMPING * dt).max(0.0);
        for index in free.clone() {
            let point = self.points[index];
            let speed = (point - self.previous[index]) * keep;
            self.previous[index] = point;
            self.points[index] = point + speed + Vec3::NEG_Y * GRAVITY * dt * dt;
        }
        let pinned = [
            (0, self.socket),
            (1, self.socket + self.out * spacing),
            (2, self.socket + self.out * 2.0 * spacing),
            (last - 2, ends.end + ends.exit * 2.0 * spacing),
            (last - 1, ends.end + ends.exit * spacing),
            (last, ends.end),
        ];
        for (index, at) in pinned {
            self.previous[index] = self.points[index];
            self.points[index] = at;
        }
        let weight = |index: usize| if free.contains(&index) { 1.0 } else { 0.0 };
        for index in 0..last {
            self.stretch(index, spacing, weight(index), weight(index + 1));
        }
        let bend = BEND_COMPLIANCE / (dt * dt);
        for index in 1..last {
            self.bend(
                index,
                bend,
                [weight(index - 1), weight(index), weight(index + 1)],
            );
        }
        for index in free {
            let mut point = self.keep_out(self.points[index], ends.device);
            if point.y <= self.floor_at(point) + 1e-5 {
                let previous = &mut self.previous[index];
                let slide = (point - *previous).with_y(0.0);
                // Static friction holds a point that barely moves, the rest
                // slides on and slows down.
                if slide.length() < STICK * dt {
                    point = previous.with_y(point.y);
                } else {
                    *previous = point - slide * FRICTION;
                }
                previous.y = point.y;
            }
            self.points[index] = point;
        }
    }

    /// Keeps the points `index` and the next one `spacing` apart.
    fn stretch(&mut self, index: usize, spacing: f32, a: f32, b: f32) {
        if a + b == 0.0 {
            return;
        }
        let off = self.points[index + 1] - self.points[index];
        let length = off.length();
        if length < 1e-7 {
            return;
        }
        let fix = off / length * (length - spacing) / (a + b);
        self.points[index] += fix * a;
        self.points[index + 1] -= fix * b;
    }

    /// Straightens the bend at `index` against its neighbours, as far as
    /// `compliance` lets it.
    fn bend(&mut self, index: usize, compliance: f32, [a, b, c]: [f32; 3]) {
        let curve = self.points[index - 1] - 2.0 * self.points[index] + self.points[index + 1];
        let length = curve.length();
        let weights = a + 4.0 * b + c;
        if length < 1e-7 || weights == 0.0 {
            return;
        }
        let normal = curve / length;
        let lambda = -length / (weights + compliance);
        self.points[index - 1] += normal * lambda * a;
        self.points[index] -= normal * lambda * 2.0 * b;
        self.points[index + 1] += normal * lambda * c;
    }

    /// `point`, moved out of the device and the computer, and onto the
    /// counter.
    fn keep_out(&self, point: Vec3, device: Option<(Affine3A, Block)>) -> Vec3 {
        let mut point = point;
        for (transform, block) in device.iter().chain(&self.solids) {
            point = Self::around(point, *transform, block);
        }
        point.with_y(point.y.max(self.floor_at(point)))
    }

    /// `point`, moved out of `bounds` of a thing at `transform` through the
    /// nearest side, if it is inside.
    fn around(point: Vec3, transform: Affine3A, bounds: &Block) -> Vec3 {
        let local = transform.inverse().transform_point3(point) - bounds.center;
        let room = bounds.size / 2.0 + RADIUS - local.abs();
        if room.min_element() <= 0.0 {
            return point;
        }
        let mut moved = local;
        let axis = room.min_position();
        moved[axis] += room[axis] * local[axis].signum();
        transform.transform_point3(moved + bounds.center)
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

    fn cable(end: Vec3) -> Cable {
        let mut cable = Cable {
            socket: Vec3::new(0.0, 1.05, 0.0),
            out: Vec3::NEG_X,
            floor: 1.0,
            mat: Block::new(Vec3::new(0.2, 0.004, 0.2)).translated(Vec3::new(-0.3, 1.0, 0.0)),
            solids: Vec::new(),
            points: Vec::new(),
            previous: Vec::new(),
            length: 0.0,
            asleep: None,
            last: None,
        };
        cable.lay(end);
        cable
    }

    fn settle(cable: &mut Cable, ends: &Ends) {
        for _ in 0..600 {
            cable.frame(ends);
        }
    }

    #[test]
    fn the_cable_comes_to_rest_on_the_counter() {
        let ends = Ends {
            end: Vec3::new(-0.4, 1.01, 0.1),
            exit: Vec3::X,
            device: None,
        };
        let mut cable = cable(ends.end);
        settle(&mut cable, &ends);
        assert!(cable.asleep.is_some());
        assert_eq!(cable.points[0], cable.socket);
        assert_eq!(cable.points[POINTS - 1], ends.end);
        for point in &cable.points {
            assert!(point.y >= cable.floor_at(*point) - 1e-4, "{point}");
        }
    }

    #[test]
    fn the_cable_stays_out_of_the_device_in_any_turn() {
        let bounds = Block::new(Vec3::new(0.06, 0.02, 0.1));
        for step in 0..8 {
            let turn = Quat::from_euler(
                EulerRot::XYZ,
                step as f32 * 0.7,
                step as f32 * 1.3,
                step as f32 * 0.4,
            );
            let transform = Affine3A::from_rotation_translation(turn, Vec3::new(-0.25, 1.25, 0.15));
            let exit = turn * Vec3::NEG_Y;
            let end = transform.transform_point3(bounds.center.with_y(0.0)) + exit * 0.016;
            let ends = Ends {
                end,
                exit,
                device: Some((transform, bounds)),
            };
            let mut cable = cable(end);
            settle(&mut cable, &ends);
            for point in &cable.points[2..POINTS - 2] {
                let local = transform.inverse().transform_point3(*point) - bounds.center;
                let inside = (local.abs() - bounds.size / 2.0).max_element() < 0.0;
                assert!(!inside, "turn {step}: {point}");
            }
        }
    }

    #[test]
    fn a_point_inside_moves_out_the_nearest_side() {
        let bounds = Block::new(Vec3::new(0.1, 0.02, 0.1));
        let transform = Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let moved = Cable::around(Vec3::new(1.0, 2.018, 3.0), transform, &bounds);
        assert!(moved.y >= 2.02 + RADIUS - 1e-6, "{moved}");
        let outside = Vec3::new(1.2, 2.0, 3.0);
        assert_eq!(Cable::around(outside, transform, &bounds), outside);
    }
}
