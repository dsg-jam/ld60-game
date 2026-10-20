//! The meshes of the manuals file, a ring binder that stands closed on the
//! counter or lies open on the mat.
//!
//! The parts are laid out for the file lying open: the spine lies in the
//! middle at x = 0 with the rings on it, running along Z. The back cover lies
//! to the right and the front cover to the left. [`Pose`](super::pose::Pose)
//! folds them shut and stands the file up.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::{Finish, SurfaceMaterial};
use crate::locale::{Localized, Message};
use crate::parts::paper::{INK, PAPER, Paper};
use crate::parts::screen::UiCanvas;

pub(super) const SHEET_SIZE: Vec2 = Vec2::new(0.16, 0.22);
pub(super) const BOARD: f32 = 0.004;
/// Width and length of each cover. The covers reach past the pages, which
/// hang further out on the rings while the file is shut.
pub(super) const COVER: Vec2 = Vec2::new(0.195, 0.24);
/// Width of the spine. The rings sit on its middle.
pub(super) const SPINE: f32 = 0.05;
/// Thickness, height and depth of the closed file standing on its bottom
/// edge, with the spine towards +Z.
pub(super) const CLOSED: Vec3 = Vec3::new(SPINE + 2.0 * BOARD, COVER.y, COVER.x + BOARD);
pub(super) const RING_RADIUS: f32 = 0.018;
pub(super) const RING_WIRE: f32 = 0.0012;
/// Height of the middle of the rings above the spine.
pub(super) const RING_HEIGHT: f32 = 0.004;
/// Distance of each ring from the middle of the spine.
pub(super) const RING_SPREAD: f32 = 0.045;
/// Height of the top of the mechanism above the spine. The pages lie on it.
pub(super) const MECHANISM_TOP: f32 = 0.001;
/// How far the mechanism reaches past the rings on either side.
const MECHANISM_LIP: f32 = 0.002;
const MECHANISM_LENGTH: f32 = 0.19;
/// How deep the rings reach into the mechanism.
const RING_DIP: f32 = 0.0006;
/// Distance of the punched holes from the bound edge of the page.
pub(super) const HOLE_INSET: f32 = 0.006;
pub(super) const HOLE_RADIUS: f32 = 0.0026;
/// Camera height above the open file while reading it.
const READING_HEIGHT: f32 = 0.33;
const SPINE_LABEL: Vec2 = Vec2::new(0.17, 0.042);
pub(super) const SPINE_CANVAS: UVec2 = UVec2::new(680, 168);
const SPINE_FONT_SIZE: f32 = 78.0;
/// Height of the spine label's center above the bottom edge.
const SPINE_LABEL_Y: f32 = 0.15;
const FINGER_HOLE: Vec3 = Vec3::new(0.026, 0.003, 0.026);
const FINGER_RIM: Vec3 = Vec3::new(0.034, 0.002, 0.034);
const FINGER_HOLE_Y: f32 = 0.045;

const CLOTH: Color = Color::srgb_u8(0x2a, 0x3c, 0x5e);
pub(super) const METAL: Color = Color::srgb_u8(0xa0, 0xa4, 0xa8);
/// The mechanism is painted, so the lamp above does not glare on it.
const MECHANISM_PAINT: Color = Color::srgb_u8(0x5c, 0x60, 0x66);
const HOLE: Color = Color::srgb_u8(0x14, 0x14, 0x16);

/// The parts of the file that move as it opens and as its pages turn.
#[derive(Component, Debug)]
pub(super) struct Parts {
    /// Lies open, or stands up with the covers shut.
    pub(super) open: Entity,
    /// The open page, on top of the pages to the right.
    pub(super) sheet: Entity,
    /// The back of the page turned last, on top of the turned pages.
    pub(super) back_sheet: Entity,
    /// Turns about the spine edge of the back cover, with the rings and
    /// everything on them.
    pub(super) spine: Entity,
    /// Turns about the far edge of the spine.
    pub(super) front: Entity,
    /// The pages on either side, but for the open sheets.
    pub(super) stack: Entity,
}

impl Parts {
    /// Spawns the file with its covers shut. The stacks of pages show their
    /// edges in `edges`.
    pub(super) fn spawn(
        file: &mut ChildSpawnerCommands,
        (pages, spine): (&UiCanvas, &UiCanvas),
        edges: Handle<SurfaceMaterial>,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        let mut parts = Self {
            open: Entity::PLACEHOLDER,
            sheet: Entity::PLACEHOLDER,
            back_sheet: Entity::PLACEHOLDER,
            spine: Entity::PLACEHOLDER,
            front: Entity::PLACEHOLDER,
            stack: Entity::PLACEHOLDER,
        };
        parts.open = file
            .spawn((
                Name::new("open"),
                Transform::default(),
                Visibility::Inherited,
            ))
            .with_children(|open| {
                let back = Block::new(COVER.extend(BOARD).xzy())
                    .translated(Vec3::X * (SPINE + COVER.x) / 2.0);
                open.spawn((Shape::cuboid(&back, CLOTH), Finish::Fabric));
                parts.spine = open
                    .spawn((Self::spine_hinge(), Visibility::Inherited))
                    .with_children(|hinge| {
                        let board = Block::new(Vec3::new(SPINE, BOARD, COVER.y))
                            .translated(Vec3::new(-SPINE / 2.0, -BOARD, 0.0));
                        hinge.spawn((Shape::cuboid(&board, CLOTH), Finish::Fabric));
                        spawn_rings(hinge, meshes, materials);
                        spawn_spine_outside(hinge, spine, meshes, materials);
                        parts.stack = hinge
                            .spawn((MeshMaterial3d(edges), Transform::default()))
                            .id();
                        parts.sheet = hinge
                            .spawn(Paper::sheet(
                                pages,
                                "manuals",
                                SHEET_SIZE,
                                Vec3::ZERO,
                                meshes,
                            ))
                            .id();
                        parts.back_sheet =
                            hinge.spawn((Transform::default(), Visibility::Hidden)).id();
                        parts.front = hinge
                            .spawn((Self::front_hinge(), Visibility::Inherited))
                            .with_children(|front| {
                                let board = Block::new(COVER.extend(BOARD).xzy())
                                    .translated(Vec3::new(-COVER.x / 2.0, -BOARD, 0.0));
                                front.spawn((Shape::cuboid(&board, CLOTH), Finish::Fabric));
                            })
                            .id();
                    })
                    .id();
            })
            .id();
        parts
    }

    /// The spine turns about the top of the back cover's edge.
    pub(super) fn spine_hinge() -> Transform {
        Transform::from_xyz(SPINE / 2.0, BOARD, 0.0)
    }

    /// The front cover turns about the top of the spine's far edge.
    pub(super) fn front_hinge() -> Transform {
        Transform::from_xyz(-SPINE, 0.0, 0.0)
    }

    /// Camera pose for reading the open file, in the open part. It takes in
    /// both sides, so the tabs of turned dividers show too.
    pub(super) fn reading_view() -> Transform {
        let center = Vec3::Y * BOARD;
        Transform::from_translation(center + Vec3::Y * READING_HEIGHT)
            .looking_at(center, Vec3::NEG_Z)
    }
}

/// The middle of the rings, in the space of the spine hinge. Their axis
/// runs along Z.
pub(super) fn ring_center() -> Vec2 {
    Vec2::new(-SPINE / 2.0, RING_HEIGHT)
}

/// The punched holes of a sheet, along its bound edge from the middle of the
/// sheet. Each sits on a ring.
pub(super) fn ring_holes() -> [f32; 2] {
    [-RING_SPREAD, RING_SPREAD]
}

/// The ring mechanism on the spine. Its rings come out of the top of the
/// mechanism and go through the punched holes of the pages.
fn spawn_rings(
    hinge: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let center = ring_center();
    let low = (MECHANISM_TOP - RING_DIP - center.y) / RING_RADIUS;
    let foot = low.asin();
    let width = 2.0 * (RING_RADIUS * foot.cos() + RING_WIRE + MECHANISM_LIP);
    let mechanism = Block::between(
        Vec3::new(
            center.x - width / 2.0,
            -BOARD / 2.0,
            -MECHANISM_LENGTH / 2.0,
        ),
        Vec3::new(
            center.x + width / 2.0,
            MECHANISM_TOP,
            MECHANISM_LENGTH / 2.0,
        ),
    );
    hinge.spawn((Shape::cuboid(&mechanism, MECHANISM_PAINT), Finish::Paint));
    let ring = meshes.add(
        Torus {
            minor_radius: RING_WIRE,
            major_radius: RING_RADIUS,
        }
        .mesh()
        .angle_range(foot..=PI - foot)
        .build(),
    );
    let metal = materials.add(StandardMaterial {
        base_color: METAL,
        metallic: 1.0,
        perceptual_roughness: 0.35,
        ..default()
    });
    for z in ring_holes() {
        hinge.spawn((
            Mesh3d(ring.clone()),
            MeshMaterial3d(metal.clone()),
            Transform::from_translation(center.extend(z))
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ));
    }
}

/// The label and the finger hole on the outside of the spine. They are
/// placed for the file standing closed, where they show.
fn spawn_spine_outside(
    hinge: &mut ChildSpawnerCommands,
    spine: &UiCanvas,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let closed = Transform::from_translation(Parts::spine_hinge().translation)
        .with_rotation(Quat::from_rotation_z(-FRAC_PI_2));
    let to_spine = |standing: Transform| {
        GlobalTransform::from(standing)
            .reparented_to(&GlobalTransform::from(super::pose::standing() * closed))
    };
    let front = CLOSED.z / 2.0;
    hinge.spawn((
        Mesh3d(meshes.add(Rectangle::from_size(SPINE_LABEL))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(spine.image.clone()),
            perceptual_roughness: 0.9,
            ..default()
        })),
        to_spine(
            Transform::from_xyz(0.0, SPINE_LABEL_Y, front + 0.0005)
                .with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
        ),
    ));
    hinge
        .spawn((
            to_spine(
                Transform::from_xyz(0.0, FINGER_HOLE_Y, front)
                    .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
            ),
            Visibility::Inherited,
        ))
        .with_children(|hole| {
            hole.spawn((
                Shape::cylinder(&Block::new(FINGER_RIM), METAL),
                Finish::Metal,
            ));
            hole.spawn((
                Shape::cylinder(&Block::new(FINGER_HOLE), HOLE),
                Finish::Plastic,
            ));
        });
}

/// Writes the label on the spine.
pub(super) fn write_spine(commands: &mut Commands, spine: &UiCanvas) {
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(PAPER),
        spine.target(),
        children![(
            Localized::from(Message::new("manuals-spine")),
            TextFont {
                font_size: FontSize::Px(SPINE_FONT_SIZE),
                ..default()
            },
            TextColor(INK),
        )],
    ));
}
