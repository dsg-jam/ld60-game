//! The pages and divider cards on the rings. The pages not turned yet lie
//! face up to the right of the rings and the turned ones face down to the
//! left, each stack with its sheets a little out of line.

use std::f32::consts::{FRAC_PI_2, PI};
use std::ops::Range;

use bevy::{platform::hash::fixed_hash_one, prelude::*};

use super::dividers::DividerTab;
use super::model::{BOARD, HOLE_RADIUS, Parts, SHEET_SIZE, holes, page_x};
use super::turning::LeafKit;

/// Thickness of a page, a little more than real paper so the stack shows
/// each sheet.
const PAGE: f32 = 0.0003;
/// Thickness of a divider card.
pub(super) const CARD: f32 = 0.0009;
/// How far the edges of a sheet stray from the others at most.
const STAGGER: Vec2 = Vec2::new(0.0008, 0.0006);
/// Keeps the dark in a punched hole off the sheet below.
const HOLE_LIFT: f32 = 0.00005;
const HOLE_SEGMENTS: u32 = 16;

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
    /// How many items lie turned to the left.
    turned: usize,
    pub(super) turning: Option<Turning>,
}

/// Items turning over together, from the stack on one side to the other.
#[derive(Debug)]
pub(super) struct Turning {
    pub(super) items: Range<usize>,
    pub(super) forward: bool,
    /// The turning leaf, see [`Leaf`](crate::parts::paper::Leaf).
    pub(super) leaf: Entity,
}

impl Stack {
    /// `count` pages with `cards` in front of them, each in front of the
    /// page it starts at. All of them lie to the right.
    pub(super) fn new(count: usize, cards: &[(usize, Entity)]) -> Self {
        let items = (0..count)
            .flat_map(|page| {
                cards
                    .iter()
                    .filter(move |(start, _)| *start == page)
                    .map(|&(_, card)| Item::Card(card))
                    .chain([Item::Page(page)])
            })
            .collect();
        Self {
            items,
            turned: 0,
            turning: None,
        }
    }

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
        match self.items[index] {
            Item::Page(_) => PAGE,
            Item::Card(_) => CARD,
        }
    }

    /// Height of all items stacked up.
    fn depth(&self) -> f32 {
        (0..self.items.len())
            .map(|index| self.thickness(index))
            .sum()
    }

    /// The items turn about this line along Z, halfway up the stack. Turning
    /// about it lays an item from the right on the same spot on the left.
    pub(super) fn pivot(&self) -> Vec3 {
        Vec3::Y * (BOARD + self.depth() / 2.0)
    }

    /// Where item `index` lies face up on the right: the middle of its
    /// underside.
    pub(super) fn place(&self, index: usize) -> Vec3 {
        let above: f32 = (0..=index).map(|other| self.thickness(other)).sum();
        let stray = stagger(index);
        Vec3::new(page_x() + stray.x, BOARD + self.depth() - above, stray.y)
    }

    /// How page `index` lies face up on the right, for a sheet from
    /// [`Paper::sheet`](crate::parts::paper::Paper::sheet).
    pub(super) fn sheet_pose(&self, index: usize) -> Transform {
        Transform::from_translation(self.place(index) + Vec3::Y * PAGE / 2.0)
            .with_rotation(Quat::from_rotation_x(-FRAC_PI_2))
    }

    /// Turns `transform` on the right over the rings to the left.
    pub(super) fn face_down(&self, transform: Transform) -> Transform {
        let pivot = self.pivot();
        Transform::from_translation(pivot)
            * Transform::from_rotation(Quat::from_rotation_z(PI))
            * Transform::from_translation(-pivot)
            * transform
    }

    /// The edges of the pages to the right, without the open page on top,
    /// and the holes under it.
    fn right_meshes(&self, right: &Range<usize>) -> (Mesh, Mesh) {
        let pages = self.pages_mesh(right.start + 1..right.end);
        let holes = self.holes_mesh(right.start, self.place(right.start).y + HOLE_LIFT, false);
        (pages, holes)
    }

    /// The edges of the pages to the left, without the back of a page on
    /// top, and the holes on top.
    fn left_meshes(&self, left: &Range<usize>, back_on_top: bool) -> Option<(Mesh, Mesh)> {
        let top = left.end.checked_sub(1)?;
        let pages = self.pages_mesh(if back_on_top { 0..top } else { 0..left.end });
        // Built face up on the right, the underside of the top item becomes
        // its upper side on the left.
        let place = self.place(top);
        let height = if back_on_top {
            place.y + self.thickness(top) - HOLE_LIFT
        } else {
            place.y - HOLE_LIFT
        };
        let holes = self.holes_mesh(top, height, true);
        let down = self.face_down(Transform::IDENTITY);
        Some((pages.transformed_by(down), holes.transformed_by(down)))
    }

    fn pages_mesh(&self, items: Range<usize>) -> Mesh {
        let mut mesh = empty_mesh();
        for index in items {
            if let Item::Page(_) = self.items[index] {
                let shade = 0.9 + 0.1 * random(index, 2);
                let slab = Cuboid::new(SHEET_SIZE.x, PAGE, SHEET_SIZE.y)
                    .mesh()
                    .build()
                    .translated_by(self.place(index) + Vec3::Y * PAGE / 2.0);
                mesh.merge(&colored(slab, shade)).ok();
            }
        }
        mesh
    }

    /// What shows through the punched holes of item `index`, at `height`
    /// and facing down if `down`.
    fn holes_mesh(&self, index: usize, height: f32, down: bool) -> Mesh {
        let mut mesh = empty_mesh();
        let place = self.place(index);
        let rotation = Quat::from_rotation_x(if down { FRAC_PI_2 } else { -FRAC_PI_2 });
        for hole in holes() {
            let at = Vec3::new(
                place.x - SHEET_SIZE.x / 2.0 + hole.x,
                height,
                place.z - SHEET_SIZE.y / 2.0 + hole.y,
            );
            let disc = Circle::new(HOLE_RADIUS)
                .mesh()
                .resolution(HOLE_SEGMENTS)
                .build()
                .transformed_by(Transform::from_translation(at).with_rotation(rotation));
            mesh.merge(&colored(disc, 1.0)).ok();
        }
        mesh
    }

    /// Lays the pages and dividers on both sides.
    #[expect(clippy::type_complexity, reason = "Bevy query")]
    pub(super) fn arrange(
        files: Query<(&Stack, &Parts, Option<&LeafKit>), Or<(Changed<Stack>, Added<LeafKit>)>>,
        dividers: Query<(Entity, &ChildOf), With<DividerTab>>,
        mut transforms: Query<&mut Transform>,
        mut visibilities: Query<&mut Visibility>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut commands: Commands,
    ) {
        for (stack, parts, kit) in &files {
            let (left, right) = stack.sides();
            if let Ok(mut sheet) = transforms.get_mut(parts.sheet) {
                *sheet = stack.sheet_pose(right.start);
            }
            let back = left
                .end
                .checked_sub(1)
                .filter(|&top| stack.page_at(top).is_some());
            if let Some(top) = back
                && let Ok(mut sheet) = transforms.get_mut(parts.back_sheet)
            {
                *sheet = stack.face_down(stack.sheet_pose(top));
                sheet.translation.y += kit.map_or(0.0, |kit| kit.shape.rise());
            }
            if let Ok(mut visibility) = visibilities.get_mut(parts.back_sheet) {
                visibility.set_if_neq(if back.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
            let sides = [
                (
                    (parts.right_stack, parts.right_holes),
                    Some(stack.right_meshes(&right)),
                ),
                (
                    (parts.left_stack, parts.left_holes),
                    stack.left_meshes(&left, back.is_some()),
                ),
            ];
            for ((edges, holes), meshes_of_side) in sides {
                match meshes_of_side {
                    Some((pages, dark)) => {
                        commands.entity(edges).insert(Mesh3d(meshes.add(pages)));
                        commands.entity(holes).insert(Mesh3d(meshes.add(dark)));
                    }
                    None => {
                        commands.entity(edges).remove::<Mesh3d>();
                        commands.entity(holes).remove::<Mesh3d>();
                    }
                }
            }
            for (index, &item) in stack.items.iter().enumerate() {
                let Item::Card(card) = item else {
                    continue;
                };
                let face_up = Transform::from_translation(stack.place(index));
                let (holder, transform) = if left.contains(&index) {
                    (parts.turned, stack.face_down(face_up))
                } else if right.contains(&index) {
                    (parts.right, face_up)
                } else {
                    continue;
                };
                if let Ok(mut current) = transforms.get_mut(card) {
                    *current = transform;
                }
                if dividers
                    .get(card)
                    .is_ok_and(|(_, parent)| parent.parent() != holder)
                {
                    commands.entity(card).insert(ChildOf(holder));
                }
            }
        }
    }
}

/// How far the edges of item `index` stray from lying flush.
fn stagger(index: usize) -> Vec2 {
    Vec2::new(random(index, 0), random(index, 1)) * 2.0 * STAGGER - STAGGER
}

/// A number from 0 to 1 that stays the same for the same `index` and `salt`.
fn random(index: usize, salt: u64) -> f32 {
    (fixed_hash_one((index, salt)) % 1024) as f32 / 1023.0
}

fn empty_mesh() -> Mesh {
    let mut mesh = Cuboid::new(0.0, 0.0, 0.0).mesh().build();
    mesh.insert_indices(bevy::mesh::Indices::U32(Vec::new()));
    colored(mesh, 1.0)
}

/// `mesh` with every vertex in `shade` of gray, so it darkens the material.
fn colored(mesh: Mesh, shade: f32) -> Mesh {
    let count = mesh.count_vertices();
    mesh.with_inserted_attribute(
        Mesh::ATTRIBUTE_COLOR,
        vec![[shade, shade, shade, 1.0]; count],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack() -> Stack {
        let cards = [
            (0, Entity::from_raw_u32(1).unwrap()),
            (1, Entity::from_raw_u32(2).unwrap()),
            (4, Entity::from_raw_u32(3).unwrap()),
        ];
        Stack::new(6, &cards)
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
    fn turning_over_lays_an_item_on_the_same_spot() {
        let stack = stack();
        let last = stack.items().len() - 1;
        let top = stack.face_down(Transform::from_translation(stack.place(0)));
        assert!((top.translation.y - BOARD - CARD).abs() < 1e-6);
        let bottom = stack.place(last);
        assert!((bottom.y - BOARD).abs() < 1e-6);
    }

    #[test]
    fn pages_of_the_open_file_lie_on_their_side() {
        let mut stack = stack();
        stack.jump_to(4);
        let (left, right) = stack.sides();
        assert_eq!(left, 0..7);
        assert_eq!(right, 7..9);
    }
}
