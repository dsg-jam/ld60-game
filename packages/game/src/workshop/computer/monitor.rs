use bevy::prelude::*;

use super::{CASE, CASE_DARK};
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;

const STAND_SIZE: Vec3 = Vec3::new(0.2, 0.03, 0.2);
const BODY_SIZE: Vec3 = Vec3::new(0.38, 0.32, 0.36);
/// The tapered back that holds the tube.
const NECK_SIZE: Vec3 = Vec3::new(0.28, 0.24, 0.12);
/// Visible glass, 4:3.
pub(super) const GLASS_SIZE: Vec2 = Vec2::new(0.3, 0.225);
pub(super) const GLASS_BULGE: f32 = 0.008;
/// Keeps the glass edge from fighting with the body front.
const GLASS_GAP: f32 = 0.0005;
const BEZEL_WIDTH: f32 = 0.022;
const BEZEL_DEPTH: f32 = 0.016;
/// How far the bezel reaches over the edge of the glass.
const BEZEL_OVERLAP: f32 = 0.008;

/// A CRT monitor on a round stand, in its own local space.
pub(super) struct Monitor {
    stand: Block,
    pub(super) body: Block,
    neck: Block,
    /// Flat bounds of the glass. The mesh bulges forward from here.
    pub(super) glass: Block,
    bezel: [Block; 4],
}

impl Monitor {
    pub(super) fn new() -> Self {
        let stand = Block::new(STAND_SIZE);
        let body = Block::new(BODY_SIZE).on_top_of(&stand);
        let neck = Block::new(NECK_SIZE).centered_on(&body).behind(&body);
        let glass = Block::new(GLASS_SIZE.extend(0.0))
            .centered_on(&body)
            .in_front_of(&body)
            .translated(Vec3::Z * GLASS_GAP);

        let opening =
            Block::new((GLASS_SIZE - 2.0 * BEZEL_OVERLAP).extend(0.0)).centered_on(&glass);
        let rail = Vec3::new(opening.size.x + 2.0 * BEZEL_WIDTH, BEZEL_WIDTH, BEZEL_DEPTH);
        let post = Vec3::new(BEZEL_WIDTH, opening.size.y, BEZEL_DEPTH);
        let bar = |size: Vec3| Block::new(size).centered_on(&opening).in_front_of(&body);
        let bezel = [
            bar(rail).on_top_of(&opening),
            bar(rail).below(&opening),
            bar(post).left_of(&opening),
            bar(post).right_of(&opening),
        ];

        Self {
            stand,
            body,
            neck,
            glass,
            bezel,
        }
    }

    /// Spawns the monitor. `glass` is the screen surface, placed onto the
    /// glass bounds.
    pub(super) fn spawn(
        &self,
        parent: &mut ChildSpawnerCommands,
        transform: Transform,
        glass: impl Bundle,
    ) {
        parent
            .spawn((Name::new("monitor"), transform, Visibility::default()))
            .with_children(|monitor| {
                monitor.spawn((Shape::cylinder(&self.stand, CASE_DARK), Finish::Plastic));
                monitor.spawn((Shape::cuboid(&self.body, CASE), Finish::Plastic));
                monitor.spawn((Shape::cuboid(&self.neck, CASE_DARK), Finish::Plastic));
                for bar in &self.bezel {
                    monitor.spawn((Shape::cuboid(bar, CASE_DARK), Finish::Plastic));
                }
                monitor.spawn((glass, self.glass.transform()));
            });
    }
}
