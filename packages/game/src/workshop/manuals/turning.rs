//! Turning the leaves of the file, with the print on the turning page.
//!
//! The open page draws into the pages canvas, which the sheet on the right
//! shows. While a page turns, the page on its front draws into a second
//! canvas, which the leaf shows. That canvas only renders while a leaf
//! with print on it turns.

use bevy::prelude::*;

use super::Filed;
use super::model::{HOLE_RADIUS, Parts, holes};
use super::pose::Pose;
use super::stack::{Item, Stack, Turning};
use crate::parts::paper::{
    Leaf, LeafShape, LeafSpec, Paper, PaperMaterial, Stiffness, Wrinkles, blank_print, reprinted,
};

/// What the file needs to turn its leaves.
#[derive(Component, Debug)]
pub(super) struct LeafKit {
    pub(super) shape: LeafShape,
    /// The page on the front of a turning leaf.
    front: Handle<PaperMaterial>,
    /// The blank back of a turning page.
    back: Handle<PaperMaterial>,
    /// The blank back of the page on top of the turned pages.
    back_sheet: Handle<PaperMaterial>,
}

impl LeafKit {
    /// Punches the open page and makes the kit from it, once the page has
    /// its material.
    pub(super) fn make(
        files: Query<(Entity, &Parts, &Filed), Without<LeafKit>>,
        sheets: Query<(&Wrinkles, &MeshMaterial3d<PaperMaterial>)>,
        mut materials: ResMut<Assets<PaperMaterial>>,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut commands: Commands,
    ) {
        for (file, parts, filed) in &files {
            let Ok((wrinkles, material)) = sheets.get(parts.sheet) else {
                continue;
            };
            let sheet = {
                let Some(mut sheet) = materials.get_mut(material) else {
                    continue;
                };
                sheet
                    .extension
                    .punch(holes().map(|hole| hole.extend(HOLE_RADIUS)));
                (*sheet).clone()
            };
            let blank = blank_print(&mut images);
            let kit = Self {
                shape: LeafShape::new(wrinkles, &mut meshes),
                front: materials.add(reprinted(&sheet, filed.leaf_pages.image.clone())),
                back: materials.add(reprinted(&sheet, blank.clone())),
                back_sheet: materials.add(reprinted(&sheet, blank)),
            };
            commands.entity(parts.back_sheet).insert((
                Mesh3d(kit.shape.back()),
                MeshMaterial3d(kit.back_sheet.clone()),
            ));
            commands.entity(file).insert(kit);
        }
    }
}

impl Stack {
    /// Turns towards the open page of the paper one leaf at a time while
    /// the file lies open, and jumps there while it is shut.
    pub(super) fn turn(
        mut files: Query<(&Paper, &mut Stack, &Pose, &Parts, &LeafKit)>,
        leaves: Query<(), With<Leaf>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut commands: Commands,
    ) {
        for (paper, mut stack, pose, parts, kit) in &mut files {
            if let Some(turning) = &stack.turning {
                if leaves.contains(turning.leaf) {
                    continue;
                }
                stack.land();
            }
            let page = paper.current_page();
            if !pose.is_open() {
                if !stack.is_at(page) {
                    stack.jump_to(page);
                }
                continue;
            }
            let Some((items, forward)) = stack.next_turn(page) else {
                continue;
            };
            let pivot = stack.pivot();
            let riders: Vec<_> = items
                .clone()
                .filter_map(|index| match stack.items()[index] {
                    Item::Card(card) => Some((
                        card,
                        Transform::from_translation(stack.place(index) - pivot),
                    )),
                    Item::Page(_) => None,
                })
                .collect();
            let card = !riders.is_empty();
            let leaf = Leaf::spawn(
                &mut commands,
                parts.open,
                &kit.shape,
                LeafSpec {
                    pivot: Transform::from_translation(pivot),
                    at: stack.sheet_pose(items.start).translation - pivot,
                    forward,
                    stiffness: if card {
                        Stiffness::Card
                    } else {
                        Stiffness::Paper
                    },
                    front: kit.front.clone(),
                    back: (!card).then(|| kit.back.clone()),
                    riders,
                },
                &mut meshes,
            );
            stack.turning = Some(Turning {
                items,
                forward,
                leaf,
            });
        }
    }

    /// Shows the open page on the sheet to the right, and the page on a
    /// turning leaf on the second canvas. The other pages hide.
    #[expect(clippy::type_complexity, reason = "Bevy query")]
    pub(super) fn show_pages(
        files: Query<
            (&Paper, &Stack, &Parts, &Filed, &LeafKit),
            Or<(Changed<Stack>, Added<LeafKit>)>,
        >,
        mut nodes: Query<(&mut Node, &mut UiTargetCamera)>,
        mut cameras: Query<&mut Camera>,
        sheets: Query<&MeshMaterial3d<PaperMaterial>>,
        mut materials: ResMut<Assets<PaperMaterial>>,
    ) {
        for (paper, stack, parts, filed, kit) in &files {
            let (left, right) = stack.sides();
            let open = stack.page_at(right.start);
            let turning = stack.turning.as_ref();
            let front = turning.and_then(|turning| stack.page_at(turning.items.start));
            let back = turning.and_then(|turning| stack.page_at(turning.items.end - 1));
            let top = left.end.checked_sub(1).and_then(|top| stack.page_at(top));
            for index in 0..paper.page_count() {
                let Some((mut node, mut target)) =
                    paper.page(index).and_then(|page| nodes.get_mut(page).ok())
                else {
                    continue;
                };
                let (camera, display) = if open == Some(index) {
                    (filed.pages.camera, Display::Flex)
                } else if front == Some(index) {
                    (filed.leaf_pages.camera, Display::Flex)
                } else {
                    (filed.pages.camera, Display::None)
                };
                if target.0 != camera {
                    target.0 = camera;
                }
                if node.display != display {
                    node.display = display;
                }
            }
            if let Ok(mut camera) = cameras.get_mut(filed.leaf_pages.camera) {
                camera.is_active = front.is_some();
            }
            let pages = [
                (sheets.get(parts.sheet).ok().map(|m| m.0.id()), open),
                (Some(kit.front.id()), front),
                (Some(kit.back.id()), back),
                (Some(kit.back_sheet.id()), top),
            ];
            for (material, page) in pages {
                if let (Some(material), Some(page)) = (material, page)
                    && let Some(mut material) = materials.get_mut(material)
                {
                    material.extension.set_page(page);
                }
            }
        }
    }
}
