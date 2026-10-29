//! The meshes of the manuals file, a ring binder that stands closed on the
//! counter or lies open on the mat.
//!
//! The parts are laid out for the file lying open: the spine lies in the
//! middle at x = 0 with the rings on it, running along Z. The back cover lies
//! to the right and the front cover to the left. [`Pose`](super::pose::Pose)
//! folds them shut and stands the file up. The cloth bends round where the
//! covers meet the spine.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use ld60_game_parts::paper::{INK, PAPER, Paper};
use ld60_game_parts::screen::UiCanvas;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::{Finish, Form, SurfaceMaterial};
use ld60_locale::{Localized, Message};

use super::cloth;
use super::mechanism::Mechanism;
use super::top_sheet::TopSheet;

pub(super) const SHEET_SIZE: Vec2 = Vec2::new(0.16, 0.22);
pub(super) const BOARD: f32 = 0.004;
/// Width and length of each cover. The covers reach past the pages, which
/// hang further out on the rings while the file is shut.
pub(super) const COVER: Vec2 = Vec2::new(0.195, 0.24);
/// Width of the spine, a little more than the mechanism. The rings sit on
/// its middle.
pub(super) const SPINE: f32 = 0.04;
/// Radius of the rounded corners at the free edge of a cover.
const COVER_CORNER: f32 = 0.006;
/// Thickness, height and depth of the closed file standing on its bottom
/// edge, with the spine towards +Z.
pub(super) const CLOSED: Vec3 = Vec3::new(SPINE + 2.0 * BOARD, COVER.y, COVER.x + BOARD);
pub(super) const RING_RADIUS: f32 = 0.015;
pub(super) const RING_WIRE: f32 = 0.0012;
/// Height of the middle of the rings above the spine.
pub(super) const RING_HEIGHT: f32 = 0.004;
/// Distance of each ring from the middle of the spine.
pub(super) const RING_SPREAD: f32 = 0.045;
/// Height of the top of the mechanism above the spine. The pages lie on it.
pub(super) const MECHANISM_TOP: f32 = 0.001;
/// Distance of the punched holes from the bound edge of the page.
pub(super) const HOLE_INSET: f32 = 0.006;
pub(super) const HOLE_RADIUS: f32 = 0.0026;
/// Camera height above the open file while reading it.
const READING_HEIGHT: f32 = 0.33;
const SPINE_LABEL: Vec2 = Vec2::new(0.17, 0.03);
pub(super) const SPINE_CANVAS: UVec2 = UVec2::new(680, 120);
const SPINE_FONT_SIZE: f32 = 60.0;
/// Height of the spine label's center above the bottom edge.
const SPINE_LABEL_Y: f32 = 0.15;
const FINGER_HOLE_RADIUS: f32 = 0.012;
/// Thickness of the metal grommet around the finger hole.
const GROMMET_WIRE: f32 = 0.0016;
const FINGER_HOLE_Y: f32 = 0.045;

const CLOTH: Color = Color::srgb_u8(0x2a, 0x3c, 0x5e);
pub(super) const METAL: Color = Color::srgb_u8(0xa0, 0xa4, 0xa8);
const HOLE: Color = Color::srgb_u8(0x0c, 0x0c, 0x0e);

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
    /// The cloth bent round the edges of the spine, at the back cover and
    /// at the front cover, each with its mesh.
    pub(super) bends: [(Entity, Handle<Mesh>); 2],
    pub(super) mechanism: Mechanism,
}

impl Parts {
    /// Spawns the file with its covers shut. The stacks of pages show their
    /// edges in `edges`.
    pub(super) fn spawn(
        file: &mut ChildSpawnerCommands,
        (pages, spine): (&UiCanvas, &UiCanvas),
        edges: Handle<SurfaceMaterial>,
        meshes: &mut Assets<Mesh>,
        (materials, surfaces): (&mut Assets<StandardMaterial>, &mut Assets<SurfaceMaterial>),
    ) -> Self {
        let mut parts = Self {
            open: Entity::PLACEHOLDER,
            sheet: Entity::PLACEHOLDER,
            back_sheet: Entity::PLACEHOLDER,
            spine: Entity::PLACEHOLDER,
            front: Entity::PLACEHOLDER,
            stack: Entity::PLACEHOLDER,
            bends: [(); 2].map(|_| {
                (
                    Entity::PLACEHOLDER,
                    meshes.add(cloth::hinge(0.0, FRAC_PI_2, BOARD, COVER.y)),
                )
            }),
            mechanism: Mechanism::default(),
        };
        let cloth = surfaces.add(Finish::Fabric.material(CLOTH, Form::Freeform, 0.0));
        let cover = meshes.add(cloth::board(COVER.extend(BOARD).xzy(), COVER_CORNER));
        parts.open = file
            .spawn((
                Name::new("open"),
                Transform::default(),
                Visibility::Inherited,
            ))
            .with_children(|open| {
                open.spawn((
                    Mesh3d(cover.clone()),
                    MeshMaterial3d(cloth.clone()),
                    Transform::from_xyz(SPINE / 2.0, 0.0, 0.0),
                ));
                parts.spine = open
                    .spawn((Self::spine_hinge(), Visibility::Inherited))
                    .with_children(|hinge| {
                        let board = Block::new(Vec3::new(SPINE, BOARD, COVER.y))
                            .translated(Vec3::new(-SPINE / 2.0, -BOARD, 0.0));
                        hinge.spawn((Shape::cuboid(&board, CLOTH), Finish::Fabric));
                        for ((bend, mesh), at) in parts.bends.iter_mut().zip([0.0, -SPINE]) {
                            *bend = hinge
                                .spawn((
                                    Mesh3d(mesh.clone()),
                                    MeshMaterial3d(cloth.clone()),
                                    Transform::from_xyz(at, 0.0, 0.0),
                                ))
                                .id();
                        }
                        parts.mechanism = Mechanism::spawn(hinge, meshes, materials);
                        spawn_spine_outside(hinge, spine, meshes, materials);
                        parts.stack = hinge
                            .spawn((MeshMaterial3d(edges), Transform::default()))
                            .id();
                        parts.sheet = hinge
                            .spawn((
                                Paper::sheet(pages, "manuals", SHEET_SIZE, Vec3::ZERO, meshes),
                                TopSheet::new(true),
                            ))
                            .id();
                        parts.back_sheet = hinge
                            .spawn((
                                Mesh3d(meshes.add(Rectangle::from_size(SHEET_SIZE))),
                                TopSheet::new(false),
                                Transform::default(),
                                Visibility::Hidden,
                            ))
                            .id();
                        parts.front = hinge
                            .spawn((Self::front_hinge(), Visibility::Inherited))
                            .with_children(|front| {
                                front.spawn((
                                    Mesh3d(cover),
                                    MeshMaterial3d(cloth),
                                    Transform::from_xyz(0.0, -BOARD, 0.0)
                                        .with_rotation(Quat::from_rotation_y(PI)),
                                ));
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
            .reparented_to(&GlobalTransform::from(super::pose::standing(0.0) * closed))
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
    // A metal grommet around a hole that looks into the dark inside.
    hinge
        .spawn((
            to_spine(Transform::from_xyz(0.0, FINGER_HOLE_Y, front)),
            Visibility::Inherited,
        ))
        .with_children(|hole| {
            hole.spawn((
                Mesh3d(meshes.add(Circle::new(FINGER_HOLE_RADIUS))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: HOLE,
                    perceptual_roughness: 1.0,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.0, 0.0002),
            ));
            hole.spawn((
                Mesh3d(meshes.add(Torus {
                    minor_radius: GROMMET_WIRE,
                    major_radius: FINGER_HOLE_RADIUS + GROMMET_WIRE / 2.0,
                })),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: METAL,
                    metallic: 1.0,
                    perceptual_roughness: 0.45,
                    ..default()
                })),
                Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2))
                    .with_scale(Vec3::new(1.0, 0.5, 1.0)),
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
