//! Turning the leaves of the file, with the print on the turning page.
//!
//! The open page draws into the pages canvas, which the sheet on the right
//! shows. While a page turns, the page on its front draws into a second
//! canvas, which the leaf shows. That canvas only renders while a leaf
//! with print on it turns.

use std::f32::consts::PI;

use bevy::prelude::*;
use ld60_core::parts::paper::{
    Bulge, Leaf, LeafShape, LeafSide, LeafSpec, Paper, PaperMaterial, Print, Stiffness, Wrinkles,
    blank_print, reprinted,
};

use super::Filed;
use super::model::{HOLE_INSET, Parts};
use super::moves::SheetMoves;
use super::pose::Pose;
use super::stack::{Stack, Turning};
use super::top_sheet::TopSheet;

/// How a page of the file is wrinkled. The open sheet, the sheet on top of
/// the turned pages and a turning leaf all show it.
#[derive(Debug)]
pub(super) struct PageShape {
    pub(super) wrinkles: Wrinkles,
    pub(super) leaf: LeafShape,
}

/// The materials the file needs to turn its leaves.
#[derive(Component, Debug)]
pub(super) struct LeafKit {
    /// The page on the front of a turning leaf.
    front: Handle<PaperMaterial>,
    /// The blank back of a turning page.
    back: Handle<PaperMaterial>,
    /// The blank back of the page on top of the turned pages.
    back_sheet: Handle<PaperMaterial>,
}

impl LeafKit {
    /// Makes the kit from the material of the open page, once it has one.
    pub(super) fn make(
        files: Query<(Entity, &Parts, &Filed), Without<LeafKit>>,
        sheets: Query<&MeshMaterial3d<PaperMaterial>>,
        mut materials: ResMut<Assets<PaperMaterial>>,
        mut images: ResMut<Assets<Image>>,
        mut commands: Commands,
    ) {
        for (file, parts, filed) in &files {
            let Some(sheet) = sheets
                .get(parts.sheet)
                .ok()
                .and_then(|material| materials.get(material))
                .cloned()
            else {
                continue;
            };
            let blank = blank_print(&mut images);
            let kit = Self {
                front: materials.add(reprinted(&sheet, filed.leaf_pages.image.clone())),
                back: materials.add(reprinted(&sheet, blank.clone())),
                back_sheet: materials.add(reprinted(&sheet, blank)),
            };
            commands
                .entity(parts.back_sheet)
                .insert(MeshMaterial3d(kit.back_sheet.clone()));
            commands.entity(file).insert(kit);
        }
    }
}

impl Stack {
    /// Turns towards the open page of the paper one leaf at a time while
    /// the file lies open, and jumps there while it is shut.
    pub(super) fn turn(
        mut files: Query<(
            &Paper,
            &mut Stack,
            &Pose,
            &Parts,
            &Filed,
            &LeafKit,
            &SheetMoves,
        )>,
        leaves: Query<(), With<Leaf>>,
        tops: Query<&TopSheet>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut commands: Commands,
    ) {
        for (paper, mut stack, pose, parts, filed, kit, moves) in &mut files {
            if let Some(turning) = &stack.turning {
                let flying = leaves.contains(turning.leaf);
                if flying && pose.is_open() {
                    continue;
                }
                if flying {
                    commands.entity(turning.leaf).despawn();
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
            if moves.holds_open_page() {
                continue;
            }
            let Some((items, forward)) = stack.next_turn(page) else {
                continue;
            };
            let Some(first) = items.clone().find(|&index| stack.page_at(index).is_some()) else {
                stack.jump_to(page);
                continue;
            };
            let last = items.end - 1;
            let bundle = items.len() > 1;
            let shape = |index: usize| {
                stack
                    .page_at(index)
                    .and_then(|page| filed.shapes.get(page))
                    .map(|shape| shape.leaf.clone())
            };
            let mut sides = Vec::new();
            if first == items.start
                && let Some(shape) = shape(first)
            {
                sides.push(LeafSide {
                    shape,
                    back: false,
                    material: kit.front.clone(),
                    at: stack.middle(first),
                    bulge: if bundle { Bulge::Front } else { Bulge::Up },
                });
            }
            if let Some(shape) = shape(last) {
                sides.push(LeafSide {
                    shape,
                    back: true,
                    material: kit.back.clone(),
                    at: stack.middle(last) + Vec3::Y * stack.apart(first, last),
                    bulge: if bundle { Bulge::Back } else { Bulge::Up },
                });
            }
            let card = items.clone().any(|index| stack.page_at(index).is_none());
            let lifted = if forward {
                parts.sheet
            } else {
                parts.back_sheet
            };
            let from = if forward { 0.0 } else { PI };
            let leaf = Leaf::spawn(
                &mut commands,
                (
                    parts.spine,
                    stack.on_rings(first, pose.cover(), from).frame(),
                ),
                LeafSpec {
                    forward,
                    stiffness: if card {
                        Stiffness::Card
                    } else {
                        Stiffness::Paper
                    },
                    held: HOLE_INSET,
                    relaxed: tops.get(lifted).map_or(0.0, |top| top.relaxed),
                    sides,
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
    /// turning leaf on the second canvas. The other pages hide. Each sheet
    /// shows the wrinkles and holes of its page.
    #[expect(clippy::type_complexity, reason = "Bevy query")]
    pub(super) fn show_pages(
        files: Query<
            (&Paper, &Stack, &Parts, &Filed, &LeafKit),
            Or<(Changed<Stack>, Added<LeafKit>)>,
        >,
        mut nodes: Query<(&mut Node, &mut UiTargetCamera)>,
        mut tops: Query<&mut TopSheet>,
        mut cameras: Query<&mut Camera>,
        sheets: Query<&MeshMaterial3d<PaperMaterial>>,
        prints: Query<&Print>,
        mut materials: ResMut<Assets<PaperMaterial>>,
    ) {
        for (paper, stack, parts, filed, kit) in &files {
            let (left, right) = stack.sides();
            let page = |index: usize| stack.page_at(index).map(|page| (index, page));
            let open = page(right.start);
            let turning = stack.turning.as_ref();
            let front = turning.and_then(|turning| page(turning.items.start));
            let back = turning.and_then(|turning| page(turning.items.end - 1));
            let top = left.end.checked_sub(1).and_then(page);
            for index in 0..paper.page_count() {
                let Some((mut node, mut target)) =
                    paper.page(index).and_then(|page| nodes.get_mut(page).ok())
                else {
                    continue;
                };
                let (camera, display) = if open.map(|(_, page)| page) == Some(index) {
                    (filed.pages.camera, Display::Flex)
                } else if front.map(|(_, page)| page) == Some(index) {
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
            for (sheet, page) in [(parts.sheet, open), (parts.back_sheet, top)] {
                if let (Ok(mut sheet), Some((_, page))) = (tops.get_mut(sheet), page) {
                    sheet.show(page);
                }
            }
            let shape = |page: usize| filed.shapes.get(page);
            let pages = [
                (sheets.get(parts.sheet).ok().map(|m| m.0.id()), open),
                (Some(kit.front.id()), front),
                (Some(kit.back.id()), back),
                (Some(kit.back_sheet.id()), top),
            ];
            for (material, page) in pages {
                let (Some(material), Some((index, page))) = (material, page) else {
                    continue;
                };
                let (Some(mut material), Some(shape)) = (materials.get_mut(material), shape(page))
                else {
                    continue;
                };
                material
                    .extension
                    .set_sheet(&shape.wrinkles, stack.sheet_holes(index));
                let print = paper.page(page).and_then(|page| prints.get(page).ok());
                material
                    .extension
                    .set_print(print.copied().unwrap_or_default());
            }
        }
    }
}
