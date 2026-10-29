mod rope;
mod solver;

use std::f32::consts::FRAC_PI_2;

use bevy::{
    camera::visibility::NoFrustumCulling, math::Affine3A, prelude::*,
    transform::helper::TransformHelper,
};
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_core::layout::desk::COMPUTER_SPOT;
use ld60_game_core::player::focus::ClickTool;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use self::rope::{Rope, RopeMaterial, RopePlugin};
use self::solver::Ends;
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
/// The cable is a chain of this many points.
const POINTS: usize = 48;
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
    /// Where the points were a tick ago, to show the cable between ticks.
    ticked: Vec<Vec3>,
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
            ticked: Vec::new(),
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
    #[expect(clippy::type_complexity, reason = "Bevy system")]
    fn follow(
        headers: Query<(Entity, &ChildOf), With<Header>>,
        plugged: Query<&Casing, With<Connected>>,
        // The device may have moved this frame, after the last transform
        // propagation, so its pose comes from the transforms.
        mut places: ParamSet<(TransformHelper, Single<(&Plug, &mut Transform)>, Aim)>,
        cable: Single<(&mut Cable, &MeshMaterial3d<RopeMaterial>)>,
        files: Query<(&ShopManuals, &Pose)>,
        mut ropes: ResMut<Assets<RopeMaterial>>,
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
        let hovered = places.p2().at(ClickTool::Plug);
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
                let goal = hovered.map_or(rest.rest, |header| on(header, PLUG_HOVER));
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
        let moved = cable.advance(time.delta_secs(), &ends);
        if !moved && cable.ticked == cable.points {
            return;
        }
        if let Some(mut rope) = ropes.get_mut(material) {
            rope.extension.follow(&cable.shown(&ends), cable.length);
        }
    }
}
