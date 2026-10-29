mod rope;

use std::f32::consts::FRAC_PI_2;

use bevy::{
    camera::visibility::NoFrustumCulling, math::Affine3A, prelude::*,
    transform::helper::TransformHelper,
};
use ld60_core::layout::counter::CounterLayout;
use ld60_core::layout::desk::COMPUTER_SPOT;
use ld60_core::player::focus::ClickTool;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use self::rope::{Rope, RopeMaterial, RopePlugin};
use super::COMPUTER_YAW;
use super::computer::ComputerLayout;
use super::manuals::{Pose, ShopManuals};
use crate::device::{Aim, Casing, Connected, Header, Holdable};

/// Where the plug of the debug cable lies while it is out, see
/// [`CounterLayout::place`].
pub(super) const PLUG_SPOT: Vec2 = Vec2::new(0.36, -0.62);
pub(super) const PLUG_YAW: f32 = 0.6;
/// The cable leaves the plug at its +Z end.
pub(super) const PLUG: Vec3 = Vec3::new(0.013, 0.0055, 0.016);
/// The boot around the cable where it leaves the monitor, long along the
/// cable. It starts this far inside the monitor.
const BOOT: Vec3 = Vec3::new(0.011, 0.011, 0.014);
const BOOT_INSIDE: f32 = 0.002;
/// The flange of the boot on the side of the monitor, along the cable.
const FLANGE: f32 = 0.0035;
/// The strain relief of the boot steps down from the first radius to the
/// second at this distance from the monitor.
const BOOT_RELIEF: [f32; 2] = [0.003, 0.0023];
const BOOT_STEP: f32 = 0.0085;
/// How far over a header the pointer is on the plug waits, and how fast it
/// moves there and back.
const PLUG_HOVER: f32 = 0.015;
const PLUG_RATE: f32 = 12.0;
/// The strain relief at the +Z end of the plug, around the cable.
const PLUG_RELIEF: f32 = 0.0024;
const PLUG_RELIEF_LENGTH: f32 = 0.005;
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
/// Steps of the simulation per tick. Small steps keep it stiff and calm.
const SUBSTEPS: usize = 12;
/// The simulation moves on in ticks of this many seconds.
const TICK: f32 = 1.0 / 60.0;
/// A slow frame runs at most this many ticks and drops the rest.
const MAX_TICKS: u32 = 4;
/// How soft the cable bends, in meters per newton. Zero would be a rod.
const BEND_COMPLIANCE: f32 = 2e-7;
/// How much of its speed the cable loses per second.
const DAMPING: f32 = 8.0;
/// How much of its sliding a point on the counter keeps per step.
const FRICTION: f32 = 0.3;
/// A point on the counter slower than this, in meters per second, stops.
const STICK: f32 = 0.05;
/// The cable sleeps once no point moves farther than this in a tick.
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
        app.add_plugins(RopePlugin)
            .add_systems(Startup, Cable::spawn)
            .add_systems(Update, Cable::follow.after(Holdable::follow));
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
    /// Time the simulation still has to catch up on, less than a [`TICK`].
    behind: f32,
    /// The ends a tick ago. Each step moves them a little of the way to
    /// where they are now, so a fast turn of the device does not yank the
    /// cable or sweep through it.
    last: Option<Ends>,
}

/// The end of the cable that goes onto the [`Header`] of a device.
#[derive(Component, Debug)]
struct Plug {
    rest: Transform,
}

/// Where the cable ends at the plug, the way it leaves the plug, and the
/// box of the device it is plugged into.
#[derive(Clone, Copy, Debug)]
struct Ends {
    end: Vec3,
    exit: Vec3,
    device: Option<(Affine3A, Block)>,
    /// The manuals file while it lies open.
    file: Option<(Affine3A, Block)>,
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
            file: other.file,
        }
    }
}

impl Cable {
    fn spawn(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut ropes: ResMut<Assets<RopeMaterial>>,
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
        // The boot looks along -Z, out of the monitor.
        let along = |from: f32, to: f32| (from - BOOT_INSIDE, to - BOOT_INSIDE);
        commands
            .spawn((
                Name::new("debug cable boot"),
                Transform::from_translation(port).looking_to(out, Vec3::Y),
                Visibility::default(),
                Pickable::IGNORE,
            ))
            .with_children(|boot| {
                let (from, to) = along(0.0, FLANGE);
                let flange = Block::new(BOOT.with_z(to - from)).translated(Vec3::new(
                    0.0,
                    -BOOT.y / 2.0,
                    -(from + to) / 2.0,
                ));
                boot.spawn((Shape::cuboid(&flange, PLUG_BLACK), Finish::Plastic));
                Self::relief(boot, along(FLANGE, BOOT_STEP), BOOT_RELIEF[0], Vec3::NEG_Z);
                Self::relief(boot, along(BOOT_STEP, BOOT.z), BOOT_RELIEF[1], Vec3::NEG_Z);
            });
        let housing = PLUG.z - PLUG_RELIEF_LENGTH;
        commands
            .spawn((
                Name::new("debug cable plug"),
                Plug { rest },
                rest,
                Visibility::default(),
                Pickable::IGNORE,
            ))
            .with_children(|plug| {
                let body = Block::new(PLUG.with_z(housing)).translated(Vec3::new(
                    0.0,
                    -PLUG.y / 2.0,
                    -PLUG.z / 2.0 + housing / 2.0,
                ));
                plug.spawn((Shape::cuboid(&body, PLUG_BLACK), Finish::Plastic));
                let start = -PLUG.z / 2.0 + housing;
                Self::relief(plug, (start, PLUG.z / 2.0), PLUG_RELIEF, Vec3::Z);
            });
        let at_computer = computer.compute_affine();
        let mut cable = Self {
            socket: port + out * (BOOT.z - BOOT_INSIDE),
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
            behind: 0.0,
            last: None,
        };
        let exit = rest.rotation * Vec3::Z;
        cable.lay(rest.translation + exit * (PLUG.z / 2.0));
        let material = ropes.add(Rope::material(
            CABLE_GRAY,
            RADIUS,
            &cable.points,
            cable.length,
        ));
        // The shader moves the tube far from the bounds of its mesh.
        commands.spawn((
            Name::new("debug cable"),
            cable,
            Mesh3d(meshes.add(Rope::mesh())),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            Visibility::Inherited,
            NoFrustumCulling,
            Pickable::IGNORE,
        ));
    }

    /// A round strain relief around the cable, from `from` to `to` along
    /// `axis`.
    fn relief(parent: &mut ChildSpawnerCommands, (from, to): (f32, f32), radius: f32, axis: Vec3) {
        let tube = Block::new(Vec3::new(2.0 * radius, to - from, 2.0 * radius));
        parent
            .spawn((
                Transform::from_translation(axis * from)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, axis)),
                Visibility::default(),
            ))
            .with_child((Shape::cylinder(&tube, PLUG_BLACK), Finish::Plastic));
    }

    /// Puts the plug on the header of the plugged in device, over a header
    /// the pointer is on, or back on the counter, and lets the cable move to
    /// it.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn follow(
        headers: Query<(Entity, &ChildOf), With<Header>>,
        plugged: Query<&Casing, With<Connected>>,
        // The device may have moved this frame, after the last transform
        // propagation, so its pose comes from the transforms.
        mut places: ParamSet<(TransformHelper, Single<(&Plug, &mut Transform)>)>,
        cable: Single<(&mut Cable, &MeshMaterial3d<RopeMaterial>)>,
        files: Query<(&ShopManuals, &Pose)>,
        mut ropes: ResMut<Assets<RopeMaterial>>,
        aim: Aim,
        time: Res<Time>,
    ) {
        let (mut cable, material) = cable.into_inner();
        let header = headers
            .iter()
            .find(|(_, parent)| plugged.contains(parent.parent()))
            .and_then(|(header, parent)| {
                let device = parent.parent();
                let casing = plugged.get(device).ok()?;
                let at = places.p0().compute_global_transform(header).ok()?;
                let device_at = places.p0().compute_global_transform(device).ok()?;
                Some((at, device_at.affine(), *casing.bounds()))
            });
        let mut plug = places.p1();
        let (rest, transform) = &mut *plug;
        // The plug stands straight up off the board, so the cable leaves the
        // device and not along it.
        let on = |header: Transform, lift: f32| {
            let away = header.rotation * Vec3::NEG_Y;
            Transform::from_translation(header.translation + away * (PLUG.z / 2.0 + lift))
                .with_rotation(header.rotation * Quat::from_rotation_x(FRAC_PI_2))
        };
        **transform = match header {
            Some((header, _, _)) => on(header.compute_transform(), 0.0),
            None => {
                let goal = aim
                    .at(ClickTool::Plug)
                    .map_or(rest.rest, |header| on(header, PLUG_HOVER));
                let dt = time.delta_secs();
                let mut moved = **transform;
                moved
                    .translation
                    .smooth_nudge(&goal.translation, PLUG_RATE, dt);
                moved.rotation.smooth_nudge(&goal.rotation, PLUG_RATE, dt);
                moved
            }
        };
        let exit = transform.rotation * Vec3::Z;
        let ends = Ends {
            end: transform.translation + exit * (PLUG.z / 2.0),
            exit,
            device: header.map(|(_, device, bounds)| (device, bounds)),
            file: files
                .iter()
                .find(|(_, pose)| pose.lying() > 0.5)
                .map(|(file, _)| (file.open_at.compute_affine(), Block::new(ShopManuals::OPEN))),
        };
        if !cable.advance(time.delta_secs(), &ends) {
            return;
        }
        if let Some(mut rope) = ropes.get_mut(material) {
            rope.extension.follow(&cable.points, cable.length);
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

    /// Moves the cable on by `dt` seconds in whole ticks. Returns whether it
    /// moved.
    fn advance(&mut self, dt: f32, ends: &Ends) -> bool {
        self.behind += dt;
        let ticks = (self.behind / TICK) as u32;
        self.behind -= ticks as f32 * TICK;
        let mut moved = false;
        for _ in 0..ticks.min(MAX_TICKS) {
            moved |= self.tick(ends);
        }
        moved
    }

    /// Moves the cable on by one tick, unless it sleeps and the plug has not
    /// moved. Returns whether it moved.
    fn tick(&mut self, ends: &Ends) -> bool {
        let wanted = Self::length_for(self.socket, ends.end);
        // Something the cable lies on or around moving wakes it too.
        let same_solids = self
            .last
            .is_some_and(|last| last.device == ends.device && last.file == ends.file);
        if wanted == self.length
            && same_solids
            && self
                .asleep
                .is_some_and(|end| end.distance(ends.end) < SLEEP)
        {
            return false;
        }
        self.length += (wanted - self.length).clamp(-REEL * TICK, REEL * TICK);
        let before = self.points.clone();
        let from = self.last.unwrap_or(*ends);
        let mut previous = from;
        for substep in 1..=SUBSTEPS {
            let between = from.towards(ends, substep as f32 / SUBSTEPS as f32);
            self.step(TICK / SUBSTEPS as f32, &between, &previous);
            previous = between;
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

    /// One small step of `dt` seconds, from the ends `before` to `ends`.
    fn step(&mut self, dt: f32, ends: &Ends, before: &Ends) {
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
        // The middle of each piece too, so a piece cannot cut through a thin
        // thing between its points.
        for index in 2..last - 2 {
            let (a, b) = (self.points[index], self.points[index + 1]);
            let middle = (a + b) / 2.0;
            let came_from = (self.previous[index] + self.previous[index + 1]) / 2.0;
            let push = self.keep_out(middle, came_from, ends, before) - middle;
            let (a, b) = (weight(index), weight(index + 1));
            self.points[index] += push * 2.0 * a / (a + b);
            self.points[index + 1] += push * 2.0 * b / (a + b);
        }
        for index in free {
            let mut point = self.keep_out(self.points[index], self.previous[index], ends, before);
            point = point.with_y(point.y.max(self.floor_at(point)));
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

    /// `point`, moved out of the device and the computer. It was at
    /// `came_from` when the ends were `before`.
    fn keep_out(&self, point: Vec3, came_from: Vec3, ends: &Ends, before: &Ends) -> Vec3 {
        let moving = |now: Option<(Affine3A, Block)>, then: Option<(Affine3A, Block)>| {
            now.map(|(transform, block)| {
                (transform, then.map_or(transform, |(then, _)| then), block)
            })
        };
        let solids = self
            .solids
            .iter()
            .map(|&(transform, block)| (transform, transform, block));
        moving(ends.device, before.device)
            .into_iter()
            .chain(moving(ends.file, before.file))
            .chain(solids)
            .fold(point, |point, (now, then, block)| {
                let came_from = now.transform_point3(then.inverse().transform_point3(came_from));
                self.around(point, came_from, now, &block)
            })
    }

    /// `point`, moved out of `bounds` of a thing at `transform` if it is
    /// inside. It leaves through the side it came in by from `came_from`,
    /// or through the nearest side above the counter if it was inside
    /// already.
    fn around(&self, point: Vec3, came_from: Vec3, transform: Affine3A, bounds: &Block) -> Vec3 {
        let inverse = transform.inverse();
        let half = bounds.size / 2.0 + RADIUS;
        let local = inverse.transform_point3(point) - bounds.center;
        let room = half - local.abs();
        if room.min_element() <= 0.0 {
            return point;
        }
        let through = |axis: usize, side: f32| {
            let mut moved = local;
            moved[axis] = half[axis] * side.signum();
            transform.transform_point3(moved + bounds.center)
        };
        let was = inverse.transform_point3(came_from) - bounds.center;
        let out = was.abs() - half;
        if out.max_element() > 0.0 {
            let axis = out.max_position();
            return through(axis, was[axis]);
        }
        (0..3)
            .flat_map(|axis| [(axis, -1.0), (axis, 1.0)])
            .map(|(axis, side)| (half[axis] - side * local[axis], through(axis, side)))
            .filter(|(_, moved)| moved.y >= self.floor_at(*moved) - 1e-4)
            .min_by(|(a, _), (b, _)| a.total_cmp(b))
            .map_or_else(
                || {
                    let axis = room.min_position();
                    through(axis, local[axis])
                },
                |(_, moved)| moved,
            )
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
            behind: 0.0,
            last: None,
        };
        cable.lay(end);
        cable
    }

    fn settle(cable: &mut Cable, ends: &Ends) {
        for _ in 0..600 {
            cable.tick(ends);
        }
    }

    #[test]
    fn the_cable_comes_to_rest_on_the_counter() {
        let ends = Ends {
            end: Vec3::new(-0.4, 1.01, 0.1),
            exit: Vec3::X,
            device: None,
            file: None,
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
    fn the_cable_keeps_time_at_any_frame_rate_and_stops_while_paused() {
        let ends = Ends {
            end: Vec3::new(-0.4, 1.01, 0.1),
            exit: Vec3::X,
            device: None,
            file: None,
        };
        let mut slow = cable(Vec3::new(-0.2, 1.2, 0.1));
        let mut fast = cable(Vec3::new(-0.2, 1.2, 0.1));
        for _ in 0..30 {
            slow.advance(1.0 / 30.0, &ends);
        }
        for _ in 0..144 {
            fast.advance(1.0 / 144.0, &ends);
        }
        for (a, b) in slow.points.iter().zip(&fast.points) {
            assert!(a.distance(*b) < 1e-3, "{a} {b}");
        }
        let paused = slow.points.clone();
        assert!(!slow.advance(0.0, &ends));
        assert_eq!(slow.points, paused);
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
                file: None,
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

    /// Whether a point or the middle of a piece of `cable` is inside `bounds`
    /// at `transform`.
    fn inside(cable: &Cable, transform: Affine3A, bounds: &Block) -> Option<Vec3> {
        let middles = cable
            .points
            .windows(2)
            .map(|pair| (pair[0] + pair[1]) / 2.0);
        cable.points[3..POINTS - 3]
            .iter()
            .copied()
            .chain(middles.skip(3).take(POINTS - 7))
            .find(|point| {
                let local = transform.inverse().transform_point3(*point) - bounds.center;
                (local.abs() - bounds.size / 2.0).max_element() < 0.0
            })
    }

    #[test]
    fn plugging_in_keeps_the_cable_out_of_the_device_at_any_yaw() {
        let bounds = Block::new(Vec3::new(0.06, 0.02, 0.1));
        let rest = Ends {
            end: Vec3::new(-0.4, 1.01, 0.1),
            exit: Vec3::X,
            device: None,
            file: None,
        };
        for step in 0..12 {
            let yaw = Quat::from_rotation_y(step as f32 * 0.55);
            let transform = Affine3A::from_rotation_translation(yaw, Vec3::new(-0.25, 1.014, 0.05));
            let end = transform.transform_point3(Vec3::new(0.0, 0.01 + 0.008, 0.02));
            let plugged = Ends {
                end,
                exit: Vec3::Y,
                device: Some((transform, bounds)),
                file: None,
            };
            let mut cable = cable(rest.end);
            settle(&mut cable, &rest);
            for tick in 0..600 {
                cable.tick(&plugged);
                if let Some(point) = inside(&cable, transform, &bounds) {
                    panic!("yaw {step}, tick {tick}: {point}");
                }
            }
        }
    }

    #[test]
    fn turning_the_device_fast_keeps_the_cable_out_of_it() {
        let bounds = Block::new(Vec3::new(0.06, 0.02, 0.1));
        let center = Vec3::new(-0.25, 1.25, 0.15);
        let at = |angle: f32| {
            let turn = Quat::from_euler(EulerRot::XYZ, angle, angle * 1.7, angle * 0.6);
            let transform = Affine3A::from_rotation_translation(turn, center);
            let exit = turn * Vec3::NEG_Y;
            Ends {
                end: transform.transform_point3(bounds.center.with_y(-0.01)) + exit * 0.008,
                exit,
                device: Some((transform, bounds)),
                file: None,
            }
        };
        let mut cable = cable(at(0.0).end);
        settle(&mut cable, &at(0.0));
        for tick in 1..240 {
            let ends = at(tick as f32 * 0.15);
            cable.tick(&ends);
            let (transform, _) = ends.device.unwrap();
            if let Some(point) = inside(&cable, transform, &bounds) {
                panic!("tick {tick}: {point}");
            }
        }
    }

    #[test]
    fn a_resting_cable_moves_out_of_a_file_opened_under_it() {
        let mut ends = Ends {
            end: Vec3::new(-0.4, 1.01, 0.1),
            exit: Vec3::X,
            device: None,
            file: None,
        };
        let mut cable = cable(ends.end);
        settle(&mut cable, &ends);
        assert!(cable.asleep.is_some());
        let file = Block::new(Vec3::new(0.2, 0.012, 0.1));
        let transform = Affine3A::from_translation(Vec3::new(-0.2, 1.0, 0.05));
        ends.file = Some((transform, file));
        settle(&mut cable, &ends);
        assert_eq!(inside(&cable, transform, &file), None);
    }

    #[test]
    fn a_point_inside_moves_out_the_nearest_side() {
        let bounds = Block::new(Vec3::new(0.1, 0.02, 0.1));
        let transform = Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let inside = Vec3::new(1.0, 2.018, 3.0);
        let cable = cable(Vec3::ZERO);
        let moved = cable.around(inside, inside, transform, &bounds);
        assert!(moved.y >= 2.02 + RADIUS - 1e-6, "{moved}");
        let outside = Vec3::new(1.2, 2.0, 3.0);
        assert_eq!(cable.around(outside, outside, transform, &bounds), outside);
    }
}
