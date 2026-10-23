//! The pages and divider cards hanging on the rings.
//!
//! Each item hangs on the ring wire by its punched holes. Where it hangs is
//! [`OnRings`]: how far it has turned about the axis of the rings, and how
//! far its middle lies from that axis. Its holes sit where its plane
//! crosses the wire, so they stay on the wire however it turns. Lying open,
//! the pages not turned yet lie face up to the right of the rings and the
//! turned ones face down to the left, each stack resting on its cover. Shut,
//! all of them hang across the middle of the spine, spread out on the rings
//! and fanning in towards their free edges, and the covers lean in on them.

use std::f32::consts::{FRAC_PI_2, PI};
use std::ops::Range;

use bevy::{mesh::VertexAttributeValues, platform::hash::fixed_hash_one, prelude::*};
use ld60_core::parts::paper::{Leaf, PAPER_SHADE};

use super::dividers::{DividerTab, TAB_OUT};
use super::model::{
    COVER, HOLE_INSET, HOLE_RADIUS, MECHANISM_TOP, Parts, RING_HEIGHT, RING_RADIUS, RING_WIRE,
    SHEET_SIZE, SPINE, ring_center, ring_holes,
};
use super::pose::Pose;
use super::punched::punched_box;

/// Thickness of a page, a little more than real paper so the stack shows
/// each sheet.
const PAGE: f32 = 0.0003;
/// Thickness of a divider card.
pub(super) const CARD: f32 = 0.0009;
/// How far the edges of a sheet stray from the others at most.
const STRAY: Vec2 = Vec2::new(0.0008, 0.0006);
/// How many times as far apart the items hang on the rings of the shut
/// file as they lie open, at most. They fill the rings, so the file looks
/// fuller.
const SPREAD: f32 = 4.0;
/// How many times as far apart the free edges of the items are in the shut
/// file as they lie open, at most. The items fan in towards them.
const FORE_SPREAD: f32 = 1.6;
/// Room the items of the shut file leave on the rings at either end.
const RING_ROOM: f32 = 0.001;
/// Room between the items and a shut cover.
const COVER_GAP: f32 = 0.0015;
/// How far the covers lean out at most when the file is too full to shut.
const SPLAY: f32 = 0.3;

/// Something on the rings, from the front of the file to the back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Item {
    Page(usize),
    /// A divider card, the entity with its [`DividerTab`].
    Card(Entity),
}

/// What is on the rings and how far it is turned.
#[derive(Component, Debug, Default)]
pub(super) struct Stack {
    items: Vec<Item>,
    /// How far the edges of each item stray from lying flush.
    strays: Vec<Vec2>,
    /// The thickness of the items in front of each item.
    before: Vec<f32>,
    /// How many times as far apart the items hang with the file shut.
    spread: f32,
    /// How far the items of the shut file lean in, in radians for each
    /// meter they hang from the middle.
    fan: f32,
    /// How many items lie turned to the left.
    turned: usize,
    pub(super) turning: Option<Turning>,
}

/// Items turning over together, from the stack on one side to the other.
#[derive(Debug)]
pub(super) struct Turning {
    pub(super) items: Range<usize>,
    pub(super) forward: bool,
    /// The turning leaf, see [`Leaf`].
    pub(super) leaf: Entity,
}

/// Where an item hangs on the rings, in the space of the spine hinge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct OnRings {
    /// How far it has turned from lying face up to the right, in radians.
    pub(super) angle: f32,
    /// How far its middle lies from the axis of the rings, towards its
    /// front.
    pub(super) offset: f32,
}

impl OnRings {
    /// The space of the item: the origin on the line through its holes in
    /// the middle of its thickness, X across it away from the bound edge, Y
    /// out of its front and Z along the spine.
    pub(super) fn frame(self) -> Transform {
        let across = Vec2::from_angle(self.angle);
        let reach = (RING_RADIUS * RING_RADIUS - self.offset * self.offset)
            .max(0.0)
            .sqrt();
        let hole = ring_center() + across.perp() * self.offset + across * reach;
        Transform::from_translation(hole.extend(0.0))
            .with_rotation(Quat::from_rotation_z(self.angle))
    }
}

impl Stack {
    /// The pages with the seed of each, and `cards` in front of them, each
    /// in front of the page it starts at with its seed. All of them lie to
    /// the right.
    pub(super) fn new(pages: &[u64], cards: &[(usize, Entity, u64)]) -> Self {
        let mut items = Vec::new();
        let mut strays = Vec::new();
        for (page, &seed) in pages.iter().enumerate() {
            for &(_, card, seed) in cards.iter().filter(|(start, ..)| *start == page) {
                items.push(Item::Card(card));
                strays.push(stray(seed));
            }
            items.push(Item::Page(page));
            strays.push(stray(seed));
        }
        for &(_, card, seed) in cards.iter().filter(|(start, ..)| *start >= pages.len()) {
            items.push(Item::Card(card));
            strays.push(stray(seed));
        }
        let before = items
            .iter()
            .scan(0.0, |sum, item| {
                let before = *sum;
                *sum += thickness(*item);
                Some(before)
            })
            .collect();
        let mut stack = Self {
            items,
            strays,
            before,
            spread: 1.0,
            fan: 0.0,
            turned: 0,
            turning: None,
        };
        let room = 2.0 * (RING_RADIUS - RING_WIRE - RING_ROOM);
        stack.spread = (room / stack.depth()).clamp(1.0, SPREAD);
        stack.fan = (1.0 - FORE_SPREAD / stack.spread).max(0.0) / SHEET_SIZE.x;
        stack
    }

    #[cfg(test)]
    pub(super) fn items(&self) -> &[Item] {
        &self.items
    }

    pub(super) fn page_at(&self, index: usize) -> Option<usize> {
        match self.items.get(index)? {
            Item::Page(page) => Some(*page),
            Item::Card(_) => None,
        }
    }

    /// Where `page` is among the items.
    pub(super) fn index_of(&self, page: usize) -> Option<usize> {
        self.items.iter().position(|&item| item == Item::Page(page))
    }

    /// Whether `page` is open, with nothing turning.
    pub(super) fn is_at(&self, page: usize) -> bool {
        self.turning.is_none() && self.index_of(page).is_none_or(|index| index == self.turned)
    }

    /// Turns to `page` at once.
    pub(super) fn jump_to(&mut self, page: usize) {
        if let Some(index) = self.index_of(page)
            && self.turned != index
        {
            self.turned = index;
        }
    }

    /// Ends the turn of the leaf in flight, if any.
    pub(super) fn land(&mut self) {
        if let Some(turning) = self.turning.take() {
            self.turned = if turning.forward {
                turning.items.end
            } else {
                turning.items.start
            };
        }
    }

    /// The items to turn next on the way to `page`, and whether they turn
    /// forward. The pages up to a divider turn with it, like when lifting
    /// the divider. The pages after the last divider on the way turn on
    /// their own.
    pub(super) fn next_turn(&self, page: usize) -> Option<(Range<usize>, bool)> {
        let target = self.index_of(page)?;
        let forward = target > self.turned;
        let (from, to) = (self.turned.min(target), self.turned.max(target));
        if from == to {
            return None;
        }
        let card = (from..to)
            .rev()
            .find(|&index| matches!(self.items[index], Item::Card(_)));
        let items = match card {
            Some(card) if forward => from..card + 1,
            Some(card) if card + 1 < to => card + 1..to,
            _ => from..to,
        };
        Some((items, forward))
    }

    /// The items lying turned to the left and those to the right, without
    /// the ones turning.
    pub(super) fn sides(&self) -> (Range<usize>, Range<usize>) {
        match &self.turning {
            Some(turning) => (0..turning.items.start, turning.items.end..self.items.len()),
            None => (0..self.turned, self.turned..self.items.len()),
        }
    }

    fn thickness(&self, index: usize) -> f32 {
        thickness(self.items[index])
    }

    /// Height of all items stacked up.
    fn depth(&self) -> f32 {
        self.before.last().copied().unwrap_or(0.0)
            + self
                .items
                .len()
                .checked_sub(1)
                .map_or(0.0, |last| self.thickness(last))
    }

    /// How deep the middle of item `index` lies in the stack.
    fn middle_depth(&self, index: usize) -> f32 {
        self.before[index] + self.thickness(index) / 2.0
    }

    /// The offset of item `index` lying open to the right and to the left,
    /// and with the file shut.
    fn offsets(&self, index: usize) -> [f32; 3] {
        let middle = self.middle_depth(index);
        let depth = self.depth();
        [
            MECHANISM_TOP + depth - middle - RING_HEIGHT,
            RING_HEIGHT - MECHANISM_TOP - middle,
            (depth / 2.0 - middle) * self.spread,
        ]
    }

    /// Where item `index` hangs with the file shut.
    fn shut(&self, index: usize) -> OnRings {
        let [_, _, offset] = self.offsets(index);
        OnRings {
            angle: FRAC_PI_2 - self.fan * offset,
            offset,
        }
    }

    /// How far the covers of the shut file lean in from upright, in
    /// radians, so they come as close to the items as they can without
    /// reaching into them. Each cover turns about the edge of the spine in
    /// the space of the spine hinge, the back cover at x = 0 and the front
    /// cover at x = -[`SPINE`].
    pub(super) fn lean(&self) -> f32 {
        let mut slope = (SPINE / 2.0 - COVER_GAP) / COVER.x;
        for (index, &item) in self.items.iter().enumerate() {
            let frame = self.shut(index).frame();
            let tab = if matches!(item, Item::Card(_)) {
                TAB_OUT
            } else {
                0.0
            };
            let reach = self.middle(index).x + SHEET_SIZE.x / 2.0 + tab;
            let half = self.thickness(index) / 2.0;
            for face in [-half, half] {
                let point = frame.transform_point(Vec3::new(reach, face, 0.0));
                if point.y <= 0.0 {
                    continue;
                }
                let back = (-point.x - COVER_GAP) / point.y;
                let front = (point.x + SPINE - COVER_GAP) / point.y;
                slope = slope.min(back).min(front);
            }
        }
        slope.atan().max(-SPLAY)
    }

    /// Where item `index` hangs with the cover `open` from 0 shut to 1, and
    /// a leaf in flight at `turning` radians.
    pub(super) fn on_rings(&self, index: usize, open: f32, turning: f32) -> OnRings {
        let [right, left, _] = self.offsets(index);
        let shut = self.shut(index);
        let (left_side, right_side) = self.sides();
        let half = (1.0 - open) * FRAC_PI_2;
        let tilt = (1.0 - open) * (shut.angle - FRAC_PI_2);
        if left_side.contains(&index) {
            OnRings {
                angle: PI - half + tilt,
                offset: shut.offset.lerp(left, open),
            }
        } else if right_side.contains(&index) {
            OnRings {
                angle: half + tilt,
                offset: shut.offset.lerp(right, open),
            }
        } else {
            OnRings {
                angle: turning,
                offset: right.lerp(left, turning / PI),
            }
        }
    }

    /// The middle of item `index` in its [`OnRings::frame`].
    pub(super) fn middle(&self, index: usize) -> Vec3 {
        middle(self.strays[index])
    }

    /// How page item `index` lies, for a sheet from
    /// [`Paper::sheet`](ld60_core::parts::paper::Paper::sheet) in `frame`.
    pub(super) fn sheet_in(&self, index: usize, frame: Transform) -> Transform {
        frame
            * Transform::from_translation(self.middle(index))
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2))
    }

    /// Where a page hangs that goes back on the rings in front of item
    /// `index`, or behind the last item, with the cover `open` from 0 shut
    /// to 1.
    pub(super) fn before(&self, index: usize, open: f32) -> OnRings {
        let last = self.items.len().saturating_sub(1);
        let (anchor, step) = if index <= last {
            (index, 1.0)
        } else {
            (last, -1.0)
        };
        let mut on = self.on_rings(anchor, open, 0.0);
        on.offset += step * PAGE * self.spread.lerp(1.0, open);
        on
    }

    /// The holes of item `index`, from its middle.
    pub(super) fn holes(&self, index: usize) -> [Vec2; 2] {
        holes(self.strays[index])
    }

    /// The holes of page item `index`, each a center in the sheet's space
    /// and a radius, for [`Sheet::set_sheet`](ld60_core::parts::paper::Sheet).
    pub(super) fn sheet_holes(&self, index: usize) -> [Vec3; 2] {
        self.holes(index)
            .map(|hole| (hole + SHEET_SIZE / 2.0).extend(HOLE_RADIUS))
    }

    /// How far page items `a` and `b` lie apart in an open stack, from the
    /// front of `a`.
    pub(super) fn apart(&self, a: usize, b: usize) -> f32 {
        self.middle_depth(a) - self.middle_depth(b)
    }

    /// The pages in the stacks as boxes punched for the rings, without the
    /// pages the open sheets show.
    fn pages_mesh(&self, open: f32, shown: [Option<usize>; 2]) -> Mesh {
        let (left, right) = self.sides();
        let mut mesh = empty_mesh();
        for index in left.chain(right) {
            if !matches!(self.items[index], Item::Page(_)) || shown.contains(&Some(index)) {
                continue;
            }
            let shade = 0.9 + 0.1 * random(index as u64, 2);
            let slab = punched_box(
                Vec3::new(SHEET_SIZE.x, PAGE, SHEET_SIZE.y),
                &self.holes(index),
                HOLE_RADIUS,
            )
            .transformed_by(
                self.on_rings(index, open, 0.0).frame()
                    * Transform::from_translation(self.middle(index)),
            );
            mesh.merge(&colored(slab, shade)).ok();
        }
        mesh
    }

    /// Hangs the pages and dividers on the rings for the pose of the file
    /// and the leaf in flight.
    pub(super) fn arrange(
        files: Query<(Ref<Stack>, &Parts, Ref<Pose>)>,
        leaves: Query<&Leaf>,
        dividers: Query<(), With<DividerTab>>,
        mut transforms: Query<&mut Transform>,
        mut visibilities: Query<&mut Visibility>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut commands: Commands,
    ) {
        for (stack, parts, pose) in &files {
            let turning = stack.turning.as_ref().map(|turning| {
                let landed = if turning.forward { PI } else { 0.0 };
                (
                    turning,
                    leaves.get(turning.leaf).map_or(landed, Leaf::angle),
                )
            });
            if !stack.is_changed() && !pose.is_changed() && turning.is_none() {
                continue;
            }
            let open = pose.cover();
            let angle = turning.map_or(0.0, |(_, angle)| angle);
            let (left, right) = stack.sides();
            let mut place = |entity: Entity, transform: Transform| {
                if let Ok(mut current) = transforms.get_mut(entity) {
                    *current = transform;
                }
            };
            let on_top = right.clone().next().filter(|&i| stack.page_at(i).is_some());
            if let Some(top) = on_top {
                place(
                    parts.sheet,
                    stack.sheet_in(top, stack.on_rings(top, open, angle).frame()),
                );
            }
            let back = left
                .end
                .checked_sub(1)
                .filter(|&top| stack.page_at(top).is_some());
            if let Some(top) = back {
                // The back of the page lies face down, so its sheet turns
                // over.
                let frame = stack.on_rings(top, open, angle).frame();
                place(
                    parts.back_sheet,
                    frame
                        * Transform::from_translation(stack.middle(top))
                            .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
                );
            }
            if let Ok(mut visibility) = visibilities.get_mut(parts.back_sheet) {
                visibility.set_if_neq(if back.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
            if let Some((turning, angle)) = turning
                && let Some(first) = turning.items.clone().find(|&i| stack.page_at(i).is_some())
            {
                place(turning.leaf, stack.on_rings(first, open, angle).frame());
            }
            for (index, &item) in stack.items.iter().enumerate() {
                if let Item::Card(card) = item
                    && dividers.contains(card)
                {
                    let frame = stack.on_rings(index, open, angle).frame();
                    place(
                        card,
                        frame * Transform::from_translation(stack.middle(index)),
                    );
                }
            }
            let pages = stack.pages_mesh(open, [on_top, back]);
            commands
                .entity(parts.stack)
                .insert(Mesh3d(meshes.add(pages)));
        }
    }
}

/// The middle of an item whose edges `stray`, in its [`OnRings::frame`].
pub(super) fn middle(stray: Vec2) -> Vec3 {
    Vec3::new(SHEET_SIZE.x / 2.0 - HOLE_INSET + stray.x, 0.0, stray.y)
}

/// The holes of an item whose edges `stray`, from its middle.
pub(super) fn holes(stray: Vec2) -> [Vec2; 2] {
    let middle = middle(stray);
    ring_holes().map(|z| Vec2::new(-middle.x, z - middle.z))
}

fn thickness(item: Item) -> f32 {
    match item {
        Item::Page(_) => PAGE,
        Item::Card(_) => CARD,
    }
}

/// How far the edges of an item with `seed` stray from lying flush.
pub(super) fn stray(seed: u64) -> Vec2 {
    Vec2::new(random(seed, 0), random(seed, 1)) * 2.0 * STRAY - STRAY
}

/// A number from 0 to 1 that stays the same for the same `seed` and `salt`.
fn random(seed: u64, salt: u64) -> f32 {
    (fixed_hash_one((seed, salt)) % 1024) as f32 / 1023.0
}

fn empty_mesh() -> Mesh {
    let mut mesh = Cuboid::new(0.0, 0.0, 0.0).mesh().build();
    mesh.insert_indices(bevy::mesh::Indices::U32(Vec::new()));
    colored(mesh, 1.0)
}

/// `mesh` with every vertex in `shade` of gray, so it darkens the material.
/// Its faces darken further like a sheet does, so a page that peeks out
/// from under the open sheet looks the same as when it is the open sheet.
fn colored(mesh: Mesh, shade: f32) -> Mesh {
    let face = PAPER_SHADE.to_linear().red;
    let colors: Vec<[f32; 4]> = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(VertexAttributeValues::Float32x3(normals)) => normals
            .iter()
            .map(|normal| {
                let shade = if normal[1].abs() > 0.5 {
                    shade * face
                } else {
                    shade
                };
                [shade, shade, shade, 1.0]
            })
            .collect(),
        _ => vec![[shade, shade, shade, 1.0]; mesh.count_vertices()],
    };
    mesh.with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack() -> Stack {
        let cards = [
            (0, Entity::from_raw_u32(1).unwrap(), 1),
            (1, Entity::from_raw_u32(2).unwrap(), 2),
            (4, Entity::from_raw_u32(3).unwrap(), 3),
        ];
        Stack::new(&[10, 11, 12, 13, 14, 15], &cards)
    }

    /// Where the holes of item `index` lie in the space of the spine hinge.
    fn wire_holes(stack: &Stack, index: usize, on: OnRings) -> Vec<Vec3> {
        let middle = stack.middle(index);
        stack
            .holes(index)
            .map(|hole| {
                on.frame()
                    .transform_point(middle + Vec3::new(hole.x, 0.0, hole.y))
            })
            .to_vec()
    }

    #[test]
    fn cards_lie_in_front_of_their_page() {
        let stack = stack();
        assert_eq!(stack.index_of(0), Some(1));
        assert_eq!(stack.index_of(1), Some(3));
        assert_eq!(stack.index_of(4), Some(7));
        assert_eq!(stack.items().len(), 9);
    }

    #[test]
    fn pages_turn_with_the_divider_they_lie_on() {
        let mut stack = stack();
        stack.jump_to(0);
        assert_eq!(stack.next_turn(1), Some((1..3, true)));
        assert_eq!(stack.next_turn(5), Some((1..7, true)));
        stack.jump_to(5);
        assert_eq!(stack.next_turn(4), Some((7..8, false)));
        stack.jump_to(4);
        assert_eq!(stack.next_turn(3), Some((5..7, false)));
        assert_eq!(stack.next_turn(4), None);
    }

    #[test]
    fn holes_stay_on_the_wire_however_an_item_hangs() {
        let mut stack = stack();
        stack.jump_to(3);
        for index in 0..stack.items().len() {
            for open in [0.0, 0.3, 1.0] {
                for turning in [0.0, 0.7, 1.6, PI] {
                    let on = stack.on_rings(index, open, turning);
                    for hole in wire_holes(&stack, index, on) {
                        let from_axis = hole.xy() - ring_center();
                        assert!((from_axis.length() - RING_RADIUS).abs() < 1e-5);
                        assert!(ring_holes().iter().any(|z| (hole.z - z).abs() < 1e-6));
                    }
                }
            }
        }
    }

    #[test]
    fn open_stacks_rest_on_their_covers_in_order() {
        let mut stack = stack();
        stack.jump_to(3);
        let (left, right) = stack.sides();
        let height = |index: usize| {
            let on = stack.on_rings(index, 1.0, 0.0);
            let frame = on.frame();
            (frame.translation.y, frame.rotation * Vec3::Y)
        };
        let bottom = |range: Range<usize>, last: bool| {
            if last { range.end - 1 } else { range.start }
        };
        for (range, last, up) in [(left, false, Vec3::NEG_Y), (right, true, Vec3::Y)] {
            let (y, normal) = height(bottom(range.clone(), last));
            assert!(normal.abs_diff_eq(up, 1e-5));
            let thickness = stack.thickness(bottom(range, last));
            assert!((y - MECHANISM_TOP - thickness / 2.0).abs() < 1e-6);
        }
    }

    #[test]
    fn shut_covers_lean_in_without_reaching_into_the_items() {
        let stack = stack();
        let lean = stack.lean();
        assert!(lean > 0.02, "{lean}");
        let inward = Vec2::new(lean.cos(), lean.sin());
        for index in 0..stack.items().len() {
            let frame = stack.shut(index).frame();
            let reach = stack.middle(index).x + SHEET_SIZE.x / 2.0;
            for along in [0.0, reach / 2.0, reach] {
                let point = frame.transform_point(Vec3::new(along, 0.0, 0.0)).xy();
                // Distance into the file from each cover.
                let from_back = -point.dot(inward);
                let from_front =
                    (point - Vec2::new(-SPINE, 0.0)).dot(inward * Vec2::new(1.0, -1.0));
                assert!(from_back > 0.0 && from_front > 0.0, "{index} {point}");
            }
        }
    }

    #[test]
    fn the_shut_file_hangs_its_items_in_order_across_the_spine() {
        let stack = stack();
        let offsets: Vec<f32> = (0..stack.items().len())
            .map(|index| stack.on_rings(index, 0.0, 0.0).offset)
            .collect();
        assert!(offsets.windows(2).all(|pair| pair[0] > pair[1]));
        assert!((offsets[0] + offsets.last().unwrap()).abs() < 0.001 * stack.spread);
    }
}
