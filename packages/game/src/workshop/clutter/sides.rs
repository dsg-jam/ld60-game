use bevy::prelude::*;
use ld60_core::layout::counter::CounterLayout;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

#[cfg(test)]
use crate::workshop::footprint::Footprint;

/// Where things lie on the counter, see [`Block::top_spot`] of the work
/// area, and their yaw in radians. All of it outside the work area, so it
/// stays out of the way.
const SOLDERING: (Vec2, f32) = (Vec2::new(-1.3, -0.45), 0.3);
const TRAY: (Vec2, f32) = (Vec2::new(1.3, -0.5), -0.2);
const BINDER: (Vec2, f32) = (Vec2::new(1.3, 0.1), -0.1);

const STATION: Vec3 = Vec3::new(0.15, 0.08, 0.12);
const DISPLAY: Vec3 = Vec3::new(0.06, 0.025, 0.004);
const HOLDER_BASE: Vec3 = Vec3::new(0.08, 0.015, 0.08);
const HOLDER: Vec3 = Vec3::new(0.035, 0.09, 0.035);
const IRON_HANDLE: Vec3 = Vec3::new(0.022, 0.11, 0.022);
const IRON_TIP: Vec3 = Vec3::new(0.006, 0.07, 0.006);
/// How far the holder and the iron in it lean back.
const HOLDER_LEAN: f32 = 0.7;
const TRAY_SIZE: Vec3 = Vec3::new(0.16, 0.025, 0.11);
const TRAY_WALL: f32 = 0.004;
const COMPARTMENTS: UVec2 = UVec2::new(3, 2);
const SCREW: Vec3 = Vec3::new(0.006, 0.004, 0.012);
const SCREWS: usize = 7;
const BINDER_SIZE: Vec3 = Vec3::new(0.32, 0.055, 0.28);
const LABEL: Vec3 = Vec3::new(0.12, 0.002, 0.08);

const DARK_PLASTIC: Color = Color::srgb_u8(0x2c, 0x2e, 0x30);
const METAL: Color = Color::srgb_u8(0x8a, 0x8e, 0x90);
const BRASS: Color = Color::srgb_u8(0xa8, 0x88, 0x48);
const TRAY_GREY: Color = Color::srgb_u8(0x5a, 0x5e, 0x60);
const PAPER_WHITE: Color = Color::srgb_u8(0xe0, 0xda, 0xc8);
const BINDER_RED: Color = Color::srgb_u8(0x6a, 0x22, 0x20);
const DISPLAY_RED: Color = Color::srgb_u8(0xff, 0x30, 0x20);
const DISPLAY_GLOW: f32 = 4.0;

/// Spawns the things at the sides of the counter, outside the work area.
pub(super) fn spawn(
    clutter: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let counter = CounterLayout::new();
    let place = |(spot, yaw): (Vec2, f32)| (counter.place(spot, yaw), Visibility::default());
    let display = (
        Mesh3d(meshes.add(Cuboid::from_size(DISPLAY))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: DISPLAY_RED.to_linear() * DISPLAY_GLOW,
            ..default()
        })),
    );
    clutter
        .spawn(place(SOLDERING))
        .with_children(|station| spawn_soldering_station(station, display));
    clutter.spawn(place(TRAY)).with_children(spawn_tray);
    clutter.spawn(place(BINDER)).with_children(spawn_binder);
}

#[cfg(test)]
pub(super) fn footprints(counter: &CounterLayout) -> Vec<Footprint> {
    let place = |(spot, yaw): (Vec2, f32)| counter.place(spot, yaw);
    let holder = Block::new(HOLDER_BASE)
        .behind(&Block::new(STATION))
        .translated(Vec3::Z * -0.02);
    vec![
        Footprint::new("soldering station", place(SOLDERING), Block::new(STATION)),
        Footprint::new("iron holder", place(SOLDERING), holder),
        Footprint::new("screw tray", place(TRAY), Block::new(TRAY_SIZE)),
        Footprint::new("binder", place(BINDER), Block::new(BINDER_SIZE)),
    ]
}

/// A station with a temperature display and the iron in its holder.
fn spawn_soldering_station(station: &mut ChildSpawnerCommands, display: impl Bundle) {
    let base = Block::new(STATION);
    let knob = Block::new(Vec3::new(0.025, 0.012, 0.025))
        .on_top_of(&base)
        .translated(Vec3::X * 0.035);
    station.spawn((Shape::cuboid(&base, DARK_PLASTIC), Finish::Plastic));
    station.spawn((Shape::cylinder(&knob, METAL), Finish::Metal));
    station.spawn((
        display,
        Transform::from_xyz(-0.025, STATION.y * 0.6, STATION.z / 2.0 + DISPLAY.z / 2.0),
    ));

    // The holder leans back, and the iron rests in it, tip first.
    let foot = Vec3::new(0.0, 0.0, -(STATION.z / 2.0 + HOLDER_BASE.z / 2.0 + 0.02));
    let holder_base = Block::new(HOLDER_BASE).translated(foot);
    station.spawn((Shape::cylinder(&holder_base, DARK_PLASTIC), Finish::Plastic));
    let lean = Quat::from_rotation_x(-HOLDER_LEAN);
    let axis = lean * Vec3::Y;
    let holder_center = foot + Vec3::Y * HOLDER_BASE.y + axis * HOLDER.y / 2.0;
    let iron_center =
        foot + Vec3::Y * HOLDER_BASE.y + axis * (HOLDER.y * 0.6 + IRON_HANDLE.y / 2.0);
    let tip_center = iron_center - axis * (IRON_HANDLE.y + IRON_TIP.y) / 2.0;
    for (size, center, color, finish) in [
        (HOLDER, holder_center, BRASS, Finish::Metal),
        (IRON_HANDLE, iron_center, DARK_PLASTIC, Finish::Plastic),
        (IRON_TIP, tip_center, METAL, Finish::Metal),
    ] {
        let part = Block {
            center: Vec3::ZERO,
            size,
        };
        station
            .spawn((Shape::cylinder(&part, color), finish))
            .insert(Transform::from_translation(center).with_rotation(lean));
    }
}

/// A tray with a rim and dividers, and screws in the compartments.
fn spawn_tray(tray: &mut ChildSpawnerCommands) {
    let floor = Block::new(TRAY_SIZE.with_y(TRAY_WALL));
    tray.spawn((Shape::cuboid(&floor, TRAY_GREY), Finish::Plastic));
    let cell = TRAY_SIZE.xz() / COMPARTMENTS.as_vec2();
    // Walls along X, then along Z, including the dividers.
    for z in 0..=COMPARTMENTS.y {
        let wall = Block::new(Vec3::new(TRAY_SIZE.x, TRAY_SIZE.y, TRAY_WALL))
            .translated(Vec3::Z * (z as f32 * cell.y - TRAY_SIZE.z / 2.0));
        tray.spawn((Shape::cuboid(&wall, TRAY_GREY), Finish::Plastic));
    }
    for x in 0..=COMPARTMENTS.x {
        let wall = Block::new(Vec3::new(TRAY_WALL, TRAY_SIZE.y, TRAY_SIZE.z))
            .translated(Vec3::X * (x as f32 * cell.x - TRAY_SIZE.x / 2.0));
        tray.spawn((Shape::cuboid(&wall, TRAY_GREY), Finish::Plastic));
    }
    for x in 0..COMPARTMENTS.x {
        for z in 0..COMPARTMENTS.y {
            let index = (x + z * COMPARTMENTS.x) as usize;
            let center = (UVec2::new(x, z).as_vec2() + 0.5) * cell - TRAY_SIZE.xz() / 2.0;
            let color = if index.is_multiple_of(2) {
                METAL
            } else {
                BRASS
            };
            for screw in 0..SCREWS {
                // Scattered in a small heap, each turned its own way.
                let angle = (index * SCREWS + screw) as f32 * 2.4;
                let offset = Vec2::from_angle(angle)
                    * cell.min_element()
                    * 0.25
                    * ((screw % 3) as f32 / 3.0);
                let part = Block {
                    center: Vec3::ZERO,
                    size: SCREW,
                };
                let at = (center + offset)
                    .extend(TRAY_WALL + SCREW.y * (0.5 + (screw % 2) as f32))
                    .xzy();
                tray.spawn((Shape::cuboid(&part, color), Finish::Metal))
                    .insert(
                        Transform::from_translation(at).with_rotation(Quat::from_rotation_y(angle)),
                    );
            }
        }
    }
}

/// A ring binder with a label on its cover.
fn spawn_binder(binder: &mut ChildSpawnerCommands) {
    let cover = Block::new(BINDER_SIZE);
    let label = Block::new(LABEL).on_top_of(&cover);
    binder.spawn((Shape::cuboid(&cover, BINDER_RED), Finish::Plastic));
    binder.spawn((Shape::cuboid(&label, PAPER_WHITE), Finish::Paper));
}
