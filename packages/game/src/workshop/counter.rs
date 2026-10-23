use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use ld60_core::layout::counter::CounterLayout;

const SLAT: f32 = 0.12;
const SLAT_GAP: f32 = 0.008;

const STOOL_HEIGHT: f32 = 0.75;
const STOOL_SEAT: Vec3 = Vec3::new(0.34, 0.05, 0.34);
const STOOL_LEG: f32 = 0.05;
/// Stools stand this far in front of the counter, at these X positions.
const STOOL_DISTANCE: f32 = 0.35;
const STOOLS: [f32; 2] = [-0.55, 0.55];

const TOP_WOOD: Color = Color::srgb_u8(0x8e, 0x6c, 0x4c);
const SLAT_WOOD: Color = Color::srgb_u8(0x5a, 0x40, 0x2c);
const MAT: Color = Color::srgb_u8(0x4f, 0x7a, 0x6b);
const STOOL_METAL: Color = Color::srgb_u8(0x50, 0x54, 0x58);
const STOOL_SEAT_RED: Color = Color::srgb_u8(0x8a, 0x2e, 0x2a);

pub(crate) struct CounterPlugin;

impl Plugin for CounterPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Counter::on_add);
    }
}

/// The counter across the front of the shop: a wall towards the street with
/// the work surface on top, and stools for waiting customers.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Counter;

impl Counter {
    fn on_add(add: On<Add<Counter>>, mut commands: Commands) {
        let layout = CounterLayout::new();
        commands.entity(add.entity).with_children(|counter| {
            counter.spawn((Shape::cuboid(&layout.top, TOP_WOOD), Finish::Wood));
            counter.spawn((Shape::cuboid(&layout.mat, MAT), Finish::Plastic));
            counter.spawn((Shape::cuboid(&layout.shelf, TOP_WOOD), Finish::Wood));
            let wall = &layout.wall;
            let mut x = wall.min().x;
            while x < wall.max().x - SLAT_GAP {
                let width = SLAT.min(wall.max().x - x) - SLAT_GAP;
                let slat = Block::between(wall.min().with_x(x), wall.max().with_x(x + width));
                counter.spawn((Shape::cuboid(&slat, SLAT_WOOD), Finish::Wood));
                x += SLAT;
            }
            for x in STOOLS {
                let at = Vec3::new(x, 0.0, wall.min().z - STOOL_DISTANCE);
                let leg = Block::new(Vec3::new(STOOL_LEG, STOOL_HEIGHT - STOOL_SEAT.y, STOOL_LEG))
                    .translated(at);
                let seat = Block::new(STOOL_SEAT).translated(at).on_top_of(&leg);
                let foot = Block::new(Vec3::new(0.3, 0.03, 0.3)).translated(at);
                counter.spawn((Shape::cylinder(&leg, STOOL_METAL), Finish::Metal));
                counter.spawn((Shape::cylinder(&foot, STOOL_METAL), Finish::Metal));
                counter.spawn((Shape::cylinder(&seat, STOOL_SEAT_RED), Finish::Plastic));
            }
        });
    }
}
