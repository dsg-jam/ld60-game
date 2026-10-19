//! The meshes of the manuals file, standing closed or lying open.

use std::f32::consts::FRAC_PI_2;

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
const COVER: Vec2 = Vec2::new(0.185, 0.24);
/// Pages under the open one.
const STACK: Vec3 = Vec3::new(0.16, 0.012, 0.22);
/// The pages lie this far right of the middle of the back cover, to leave
/// room for the lever arch.
const STACK_SHIFT: f32 = 0.011;
const ARCH: Vec3 = Vec3::new(0.008, 0.024, 0.008);
/// From the middle of the left edge of the pages, along it.
const ARCH_SPREAD: f32 = 0.04;
const ARCH_PLATE: Vec3 = Vec3::new(0.016, 0.002, 0.12);
/// Keeps the open page from flickering against the tabs.
const SHEET_GAP: f32 = 0.0002;
pub(super) const TAB_HEIGHT: f32 = 0.0006;
/// Length of a divider tab along the right edge of the pages.
const TAB_LENGTH: f32 = 0.05;
/// How far a divider tab reaches out of the right edge of the pages.
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
/// The lever arch is painted, so the lamp above does not glare on it.
const ARCH_PAINT: Color = Color::srgb_u8(0x5c, 0x60, 0x66);
const HOLE: Color = Color::srgb_u8(0x14, 0x14, 0x16);

/// The parts of the file that show or hide when it opens.
#[derive(Component, Debug)]
pub(super) struct Parts {
    pub(super) closed: Entity,
    pub(super) open: Entity,
    pub(super) sheet: Entity,
    /// Height of the divider tabs in the open part.
    pub(super) tab_y: f32,
    /// Where the right edge of the pages is in the open part.
    pub(super) tab_x: f32,
}

/// A divider tab sticking out of the right edge of the open pages. A click
/// opens the first page of its section.
#[derive(Component, Debug)]
pub(super) struct DividerTab {
    pub(super) slot: usize,
    pub(super) rest: Vec3,
}

impl DividerTab {
    /// The canvas size for the labels of every section.
    pub(super) fn labels_size() -> UVec2 {
        TAB_LABEL * UVec2::new(Section::ALL.len() as u32, 1)
    }

    /// The tab for the section in `slot`, sticking out of the right edge of
    /// the pages in `parts`. It is the tab at `position` from the top, so
    /// the tabs stay together when a section has none.
    pub(super) fn spawn(
        open: &mut ChildSpawnerCommands,
        parts: &Parts,
        (slot, position): (usize, usize),
        labels: &UiCanvas,
    ) {
        let rest = Vec3::new(
            parts.tab_x + TAB_OUT / 2.0,
            parts.tab_y,
            -STACK.z / 2.0 + TAB_SLOT * (position as f32 + 0.5),
        );
        let count = Section::ALL.len() as f32;
        let region = Rect::new(slot as f32 / count, 0.0, (slot + 1) as f32 / count, 1.0);
        let block = Block::new(Vec3::new(TAB_LENGTH, TAB_HEIGHT, TAB_OUT));
        open.spawn((
            Self { slot, rest },
            Transform::from_translation(rest).with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
            Visibility::Inherited,
            children![(
                Shape::textured_cuboid(&block, labels.image.clone(), region),
                Finish::Paper,
            )],
        ));
    }

    /// Where the tab rests. It reaches out further while its section is
    /// open.
    pub(super) fn raised(&self, open: bool) -> Vec3 {
        if open {
            self.rest + Vec3::X * TAB_OUT * 0.4
        } else {
            self.rest
        }
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
        let mut sheet = Entity::PLACEHOLDER;
        let back = back_cover();
        let stack = stack();
        let tab_y = stack.max().y;
        let open = file
            .spawn((Name::new("open"), open, Visibility::Hidden))
            .with_children(|open| {
                let spine = Block::new(Vec3::new(CLOSED.x, BOARD, COVER.y)).left_of(&back);
                let front = back.left_of(&spine);
                for cover in [back, spine, front] {
                    open.spawn((Shape::cuboid(&cover, CLOTH), Finish::Fabric));
                }
                open.spawn((Shape::cuboid(&stack, PAPER), Finish::Paper));
                let plate = Block::new(ARCH_PLATE)
                    .on_top_of(&back)
                    .translated(Vec3::X * (back.min().x + stack.min().x) / 2.0);
                open.spawn((Shape::cuboid(&plate, ARCH_PAINT), Finish::Paint));
                for side in [-1.0, 1.0] {
                    let arch = Block::new(ARCH).on_top_of(&plate).translated(Vec3::new(
                        plate.center.x,
                        0.0,
                        side * ARCH_SPREAD,
                    ));
                    open.spawn((Shape::cylinder(&arch, ARCH_PAINT), Finish::Paint));
                }
                sheet = open
                    .spawn(Paper::sheet(
                        pages,
                        SHEET_SIZE,
                        sheet_center(stack),
                        meshes,
                        materials,
                    ))
                    .id();
            })
            .id();
        Self {
            closed,
            open,
            sheet,
            tab_y,
            tab_x: stack.max().x,
        }
    }

    /// The center of the open page, in the open part.
    pub(super) fn sheet_center() -> Vec3 {
        sheet_center(stack())
    }
}

fn back_cover() -> Block {
    Block::new(Vec3::new(COVER.x, BOARD, COVER.y))
}

/// The pages under the open one, lying on the back cover.
fn stack() -> Block {
    Block::new(STACK)
        .on_top_of(&back_cover())
        .translated(Vec3::X * STACK_SHIFT)
}

fn sheet_center(stack: Block) -> Vec3 {
    stack.center.with_y(stack.max().y + TAB_HEIGHT + SHEET_GAP)
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
