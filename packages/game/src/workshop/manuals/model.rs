//! The meshes of the manuals file, standing closed or lying open.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use super::contents::Section;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;
use crate::locale::{Localized, Message};
use crate::parts::paper::{INK, PAPER, Paper};
use crate::parts::screen::UiCanvas;

pub(super) const SHEET_SIZE: Vec2 = Vec2::new(0.16, 0.22);
/// Thickness, height and depth of the closed file standing on its bottom
/// edge, with the spine towards +Z.
pub(super) const CLOSED: Vec3 = Vec3::new(0.07, 0.25, 0.2);
const BOARD: f32 = 0.004;
const COVER: Vec2 = Vec2::new(0.2, 0.24);
/// Width of the spine of the file lying open.
const SPINE: f32 = 0.05;
/// Height of all pages stacked up.
const STACK_DEPTH: f32 = 0.01;
const RING_RADIUS: f32 = 0.02;
const RING_WIRE: f32 = 0.0012;
/// Distance of each ring from the middle of the spine edge of the pages.
const RING_SPREAD: f32 = 0.045;
/// Gap between the left end of the rings and the spine edge of the back
/// cover.
const RING_MARGIN: f32 = 0.006;
/// The rings reach a little into the mechanism at both ends.
const RING_DIP: f32 = 0.15;
const MECHANISM: Vec3 = Vec3::new(0.028, 0.004, 0.19);
/// Distance of the punched holes from the spine edge of the page.
pub(super) const HOLE_INSET: f32 = 0.006;
pub(super) const HOLE_RADIUS: f32 = 0.0025;
/// Keeps the open page from flickering against the stack.
const SHEET_GAP: f32 = 0.0002;
/// Camera height above the open file while reading it.
const READING_HEIGHT: f32 = 0.33;
/// Seconds the cover takes to open or close.
const COVER_TIME: f32 = 0.4;
const DIVIDER: f32 = 0.0005;
/// How far a divider reaches past the pages on the other edges, so its color
/// shows on the side of the stack.
const DIVIDER_LIP: f32 = 0.0006;
/// Length of a divider tab along the outer edge of the pages.
const TAB_LENGTH: f32 = 0.05;
/// How far a divider tab reaches out of the outer edge of the pages.
const TAB_OUT: f32 = 0.016;
const TAB_SLOT: f32 = 0.055;
pub(super) const TAB_LABEL: UVec2 = UVec2::new(200, 64);
const SPINE_LABEL: Vec2 = Vec2::new(0.17, 0.042);
pub(super) const SPINE_CANVAS: UVec2 = UVec2::new(680, 168);
const SPINE_FONT_SIZE: f32 = 78.0;
/// Height of the spine label's center above the bottom edge.
const SPINE_LABEL_Y: f32 = 0.15;
const FINGER_HOLE: Vec3 = Vec3::new(0.026, 0.003, 0.026);
const FINGER_RIM: Vec3 = Vec3::new(0.034, 0.002, 0.034);
const FINGER_HOLE_Y: f32 = 0.045;
/// Divider edges showing at the front edge of the closed file.
const DIVIDER_EDGE: Vec3 = Vec3::new(0.0015, 0.045, 0.008);

const CLOTH: Color = Color::srgb_u8(0x2a, 0x3c, 0x5e);
const METAL: Color = Color::srgb_u8(0xa0, 0xa4, 0xa8);
/// The mechanism is painted, so the lamp above does not glare on it.
const MECHANISM_PAINT: Color = Color::srgb_u8(0x5c, 0x60, 0x66);
const HOLE: Color = Color::srgb_u8(0x14, 0x14, 0x16);

/// The parts of the file that show, hide or move when it opens.
#[derive(Component, Debug)]
pub(super) struct Parts {
    pub(super) closed: Entity,
    pub(super) open: Entity,
    pub(super) sheet: Entity,
    /// Turns about the spine edge of the back cover.
    spine: Entity,
    /// Turns about the far edge of the spine.
    front: Entity,
    /// Turns the turned pages back over the rings as the cover closes.
    turning: Entity,
    /// Holds the turned pages.
    turned: Entity,
    stack: Entity,
    turned_stack: Entity,
}

/// How far the front cover is open, from 0 to 1.
#[derive(Component, Debug, Default)]
pub(super) struct Cover(f32);

/// A divider sheet on the rings with a tab sticking out of the outer edge of
/// the pages. A click on the tab opens the first page of its section.
#[derive(Component, Debug)]
pub(super) struct DividerTab {
    pub(super) slot: usize,
    pub(super) file: Entity,
    /// The page the divider lies on top of.
    pub(super) start: usize,
    tab: Entity,
    /// Where the tab sits along the edge.
    along: f32,
}

impl DividerTab {
    /// The canvas size for the labels of every section.
    pub(super) fn labels_size() -> UVec2 {
        TAB_LABEL * UVec2::new(Section::ALL.len() as u32, 1)
    }

    /// The divider for the section in `slot`, lying on top of page `start`.
    /// Its tab is the one at `position` from the top, so the tabs stay
    /// together when a section has none.
    pub(super) fn spawn(
        open: &mut ChildSpawnerCommands,
        file: Entity,
        (slot, position, start): (usize, usize, usize),
        labels: &UiCanvas,
    ) {
        let section = Section::ALL[slot];
        let count = Section::ALL.len() as f32;
        let region = Rect::new(slot as f32 / count, 0.0, (slot + 1) as f32 / count, 1.0);
        let body = Block::new(Vec3::new(
            SHEET_SIZE.x + DIVIDER_LIP,
            DIVIDER,
            SHEET_SIZE.y + 2.0 * DIVIDER_LIP,
        ))
        .translated(Vec3::X * DIVIDER_LIP / 2.0);
        let label = Block::new(Vec3::new(TAB_LENGTH, DIVIDER, TAB_OUT));
        let along = -SHEET_SIZE.y / 2.0 + TAB_SLOT * (position as f32 + 0.5);
        let mut tab = Entity::PLACEHOLDER;
        let mut divider = open.spawn((Transform::default(), Visibility::Inherited));
        divider.with_children(|divider| {
            divider.spawn((Shape::cuboid(&body, section.color()), Finish::Paper));
            tab = divider
                .spawn((
                    Transform::from_xyz((SHEET_SIZE.x + TAB_OUT) / 2.0, 0.0, along)
                        .with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
                    Visibility::Inherited,
                    children![(
                        Shape::textured_cuboid(&label, labels.image.clone(), region),
                        Finish::Paper,
                    )],
                ))
                .id();
        });
        divider.insert(Self {
            slot,
            file,
            start,
            tab,
            along,
        });
    }
}

impl Parts {
    /// Spawns the file standing closed and the file lying open at `open`.
    pub(super) fn spawn(
        file: &mut ChildSpawnerCommands,
        open: Transform,
        pages: &UiCanvas,
        spine: &UiCanvas,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        let closed = file
            .spawn((
                Name::new("closed"),
                Transform::default(),
                Visibility::Inherited,
            ))
            .with_children(|closed| spawn_closed(closed, spine, meshes, materials))
            .id();
        let back = back_cover();
        let full = Block::new(SHEET_SIZE.extend(STACK_DEPTH).xzy());
        let mut parts = Self {
            closed,
            open: Entity::PLACEHOLDER,
            sheet: Entity::PLACEHOLDER,
            spine: Entity::PLACEHOLDER,
            front: Entity::PLACEHOLDER,
            turning: Entity::PLACEHOLDER,
            turned: Entity::PLACEHOLDER,
            stack: Entity::PLACEHOLDER,
            turned_stack: Entity::PLACEHOLDER,
        };
        parts.open = file
            .spawn((Name::new("open"), open, Visibility::Hidden))
            .with_children(|open| {
                open.spawn((Shape::cuboid(&back, CLOTH), Finish::Fabric));
                parts.stack = open
                    .spawn((Shape::cuboid(&full, PAPER), Finish::Paper))
                    .id();
                spawn_rings(open, meshes, materials);
                parts.sheet = open
                    .spawn(Paper::sheet(
                        pages,
                        SHEET_SIZE,
                        sheet_center(STACK_DEPTH),
                        meshes,
                    ))
                    .id();
                let pivot = turn_pivot();
                parts.turning = open
                    .spawn((Transform::from_translation(pivot), Visibility::Inherited))
                    .with_children(|turning| {
                        parts.turned = turning
                            .spawn((Transform::from_translation(-pivot), Visibility::Inherited))
                            .with_children(|turned| {
                                parts.turned_stack = turned
                                    .spawn((Shape::cuboid(&full, PAPER), Finish::Paper))
                                    .id();
                            })
                            .id();
                    })
                    .id();
                let hinge = back.min().with_y(0.0).with_z(0.0);
                parts.spine = open
                    .spawn((Transform::from_translation(hinge), Visibility::Inherited))
                    .with_children(|spine| {
                        let board = Block::new(Vec3::new(SPINE, BOARD, COVER.y))
                            .translated(Vec3::NEG_X * SPINE / 2.0);
                        spine.spawn((Shape::cuboid(&board, CLOTH), Finish::Fabric));
                        parts.front = spine
                            .spawn((Transform::from_xyz(-SPINE, 0.0, 0.0), Visibility::Inherited))
                            .with_children(|front| {
                                let board = back.translated(Vec3::NEG_X * COVER.x / 2.0);
                                front.spawn((Shape::cuboid(&board, CLOTH), Finish::Fabric));
                            })
                            .id();
                    })
                    .id();
            })
            .id();
        parts
    }

    /// Camera pose for reading the open file, in the open part. It takes in
    /// both sides, so the tabs of turned dividers show too.
    pub(super) fn reading_view() -> Transform {
        let center = sheet_center(STACK_DEPTH / 2.0).with_x(ring_center().x);
        Transform::from_translation(center + Vec3::Y * READING_HEIGHT)
            .looking_at(center, Vec3::NEG_Z)
    }

    /// The parent of the dividers in front of page `current`.
    pub(super) fn holder(&self, turned: bool) -> Entity {
        if turned { self.turned } else { self.open }
    }

    /// Stacks the pages before `current` of `count` on the front cover and
    /// the rest on the back cover, with the dividers in between.
    pub(super) fn stack_pages(
        &self,
        (current, count): (usize, usize),
        transforms: &mut Query<&mut Transform>,
        visibilities: &mut Query<&mut Visibility>,
    ) {
        let page = STACK_DEPTH / count.max(1) as f32;
        let unturned = count.saturating_sub(current) as f32 * page;
        let turned = current as f32 * page;
        for (stack, depth, x) in [
            (self.stack, unturned, stack_x()),
            (self.turned_stack, turned, turned_x()),
        ] {
            if let Ok(mut transform) = transforms.get_mut(stack) {
                transform.translation = Vec3::new(x, BOARD + depth / 2.0, 0.0);
                transform.scale = Vec3::ONE.with_y((depth / STACK_DEPTH).max(0.01));
            }
            if let Ok(mut visibility) = visibilities.get_mut(stack) {
                visibility.set_if_neq(if depth > 0.0 {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
        if let Ok(mut transform) = transforms.get_mut(self.sheet) {
            transform.translation = sheet_center(unturned);
        }
    }

    /// Opens the front cover by `step` towards `open`, from 0 to 1. Returns
    /// whether any of the file is still open.
    pub(super) fn swing_cover(
        &self,
        cover: &mut Cover,
        open: bool,
        step: f32,
        transforms: &mut Query<&mut Transform>,
    ) -> bool {
        let target = if open { 1.0 } else { 0.0 };
        let next = if cover.0 < target {
            (cover.0 + step / COVER_TIME).min(target)
        } else {
            (cover.0 - step / COVER_TIME).max(target)
        };
        if next != cover.0 {
            cover.0 = next;
            let eased = next * next * (3.0 - 2.0 * next);
            let rotation = Quat::from_rotation_z(-(1.0 - eased) * FRAC_PI_2);
            for hinge in [self.spine, self.front] {
                if let Ok(mut transform) = transforms.get_mut(hinge) {
                    transform.rotation = rotation;
                }
            }
            if let Ok(mut transform) = transforms.get_mut(self.turning) {
                transform.rotation = rotation * rotation;
            }
        }
        cover.0 > 0.0
    }
}

impl DividerTab {
    /// Lays the divider on top of page `start`. Turned dividers lie face
    /// down on the turned pages, with the tab out of their outer edge.
    pub(super) fn place(
        &self,
        (current, count): (usize, usize),
        divider: Entity,
        transforms: &mut Query<&mut Transform>,
    ) {
        let page = STACK_DEPTH / count.max(1) as f32;
        let turned = self.start <= current;
        if let Ok(mut divider) = transforms.get_mut(divider) {
            *divider = if turned {
                Transform::from_xyz(turned_x(), BOARD + self.start as f32 * page, 0.0)
                    .with_rotation(Quat::from_rotation_y(PI))
            } else {
                Transform::from_xyz(
                    stack_x(),
                    BOARD + count.saturating_sub(self.start) as f32 * page,
                    0.0,
                )
            };
        }
        if let Ok(mut tab) = transforms.get_mut(self.tab) {
            tab.translation.z = if turned { -self.along } else { self.along };
        }
    }
}

fn back_cover() -> Block {
    Block::new(Vec3::new(COVER.x, BOARD, COVER.y))
}

/// The center of the rings, which the pages turn about.
fn ring_center() -> Vec3 {
    Vec3::new(
        back_cover().min().x + RING_MARGIN + RING_RADIUS,
        BOARD + MECHANISM.y / 2.0,
        0.0,
    )
}

/// Turning the turned pages over the rings about this point lays them face
/// down on the other pages.
fn turn_pivot() -> Vec3 {
    ring_center().with_y(BOARD + STACK_DEPTH / 2.0)
}

/// The middle of the pages not yet turned.
fn stack_x() -> f32 {
    ring_center().x + RING_RADIUS - HOLE_INSET + SHEET_SIZE.x / 2.0
}

/// The middle of the turned pages.
fn turned_x() -> f32 {
    ring_center().x - RING_RADIUS + HOLE_INSET - SHEET_SIZE.x / 2.0
}

/// The center of the open page on top of pages `depth` high.
fn sheet_center(depth: f32) -> Vec3 {
    Vec3::new(stack_x(), BOARD + depth + SHEET_GAP, 0.0)
}

/// How far the rings are from the spine edge of the open page.
pub(super) fn ring_gap() -> f32 {
    RING_RADIUS - HOLE_INSET
}

/// The punched holes, from the top of the open page down.
pub(super) fn holes() -> [f32; 2] {
    [
        SHEET_SIZE.y / 2.0 - RING_SPREAD,
        SHEET_SIZE.y / 2.0 + RING_SPREAD,
    ]
}

/// The ring mechanism on the back cover. Its rings go through the punched
/// holes of the pages.
fn spawn_rings(
    open: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let center = ring_center();
    let mechanism = Block::new(MECHANISM)
        .on_top_of(&back_cover())
        .translated(Vec3::X * center.x);
    open.spawn((Shape::cuboid(&mechanism, MECHANISM_PAINT), Finish::Paint));
    let ring = meshes.add(
        Torus {
            minor_radius: RING_WIRE,
            major_radius: RING_RADIUS,
        }
        .mesh()
        .angle_range(-RING_DIP..=PI + RING_DIP)
        .build(),
    );
    let metal = materials.add(StandardMaterial {
        base_color: METAL,
        metallic: 1.0,
        perceptual_roughness: 0.35,
        ..default()
    });
    for side in [-1.0, 1.0] {
        open.spawn((
            Mesh3d(ring.clone()),
            MeshMaterial3d(metal.clone()),
            Transform::from_translation(center.with_z(side * RING_SPREAD))
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ));
    }
}

fn spawn_closed(
    closed: &mut ChildSpawnerCommands,
    spine: &UiCanvas,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    for side in [-1.0, 1.0] {
        let cover =
            Block::new(CLOSED.with_x(BOARD)).translated(Vec3::X * side * (CLOSED.x - BOARD) / 2.0);
        closed.spawn((Shape::cuboid(&cover, CLOTH), Finish::Fabric));
    }
    let back = Block::new(CLOSED.with_z(BOARD)).translated(Vec3::Z * (CLOSED.z - BOARD) / 2.0);
    closed.spawn((Shape::cuboid(&back, CLOTH), Finish::Fabric));
    let pages = Block::new(CLOSED - Vec3::new(2.0 * BOARD + 0.004, 0.02, 0.02))
        .translated(Vec3::new(0.0, BOARD, -0.004));
    closed.spawn((Shape::cuboid(&pages, PAPER), Finish::Paper));
    for (slot, color) in Section::ALL.map(Section::color).into_iter().enumerate() {
        let x = pages.min().x + pages.size.x * (slot as f32 + 0.5) / 4.0;
        let y = pages.max().y - DIVIDER_EDGE.y * (slot as f32 + 1.0);
        let edge = Block::new(DIVIDER_EDGE).translated(Vec3::new(x, y, -CLOSED.z / 2.0 - 0.001));
        closed.spawn((Shape::cuboid(&edge, color), Finish::Paper));
    }

    let front = CLOSED.z / 2.0;
    closed.spawn((
        Mesh3d(meshes.add(Rectangle::from_size(SPINE_LABEL))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(spine.image.clone()),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, SPINE_LABEL_Y, front + 0.0005)
            .with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
    ));
    closed
        .spawn((
            Transform::from_xyz(0.0, FINGER_HOLE_Y, front)
                .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
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
