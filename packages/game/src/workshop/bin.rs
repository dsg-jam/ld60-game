use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use ld60_core::audio::{Play, Sound};
use ld60_core::layout::counter::CounterLayout;
use ld60_core::layout::shop::ShopLayout;
use ld60_core::player::focus::{Clickable, FocusTarget};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

/// On the rug under the counter, left of the player's knees, see
/// [`CounterLayout::under_top`].
const ALONG: f32 = -0.42;
const UNDER: f32 = 0.06;
/// The box around the bin.
const SIZE: Vec3 = Vec3::new(0.26, 0.3, 0.26);
/// The wire basket is this many bars around.
const BARS: usize = 28;
const BAR: f32 = 0.003;
const RIM: f32 = 0.004;
const BOTTOM: f32 = 0.004;
const WIRE: Color = Color::srgb_u8(0x3a, 0x3c, 0x40);
const CRUMPLE_TIME: f32 = 0.35;
const TOSS_TIME: f32 = 0.7;
const FALL_TIME: f32 = 0.45;
/// How small a slip is once it is crumpled up.
const CRUMPLED: f32 = 0.22;
/// How high a slip arcs on its way over the edge of the counter.
const ARC: f32 = 0.1;
/// How far past the edge of the counter the slip goes before it drops.
const PAST_EDGE: f32 = 0.05;

pub(crate) struct BinPlugin;

impl Plugin for BinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Binned::toss)
            .add_observer(Bin::on_add);
    }
}

/// The wire paper bin on the counter that old receipts go into.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Bin;

/// A receipt being crumpled up and tossed into the [`Bin`]. It is gone once
/// it is in.
#[derive(Component, Debug, Default)]
pub(crate) struct Binned {
    elapsed: f32,
    from: Option<Transform>,
}

impl Bin {
    /// Where the bin stands, on the rug under the counter.
    pub(super) fn place() -> Transform {
        let mut at = CounterLayout::new().under_top(ALONG, UNDER);
        at.translation.y = ShopLayout::new().rug.max().y;
        at
    }

    fn on_add(add: On<Add<Bin>>, mut commands: Commands) {
        let radius = SIZE.x / 2.0;
        let bottom = Block::new(Vec3::new(SIZE.x, BOTTOM, SIZE.z));
        let bar = Block::new(Vec3::new(BAR, SIZE.y, BAR));
        let rim = Block::new(Vec3::new(TAU * radius / BARS as f32 + BAR, RIM, RIM))
            .translated(Vec3::Y * (SIZE.y - RIM));
        commands.entity(add.entity).with_children(|bin| {
            bin.spawn((Shape::cylinder(&bottom, WIRE), Finish::Metal));
            for index in 0..BARS {
                let angle = index as f32 / BARS as f32 * TAU;
                let turn = Quat::from_rotation_y(angle);
                // Each bar stands on the bottom, with a piece of the rim
                // from it to the next one.
                bin.spawn((
                    Transform::from_rotation(turn).with_translation(turn * Vec3::Z * radius),
                    Visibility::Inherited,
                ))
                .with_children(|bars| {
                    bars.spawn((Shape::cylinder(&bar, WIRE), Finish::Metal));
                });
                let half = Quat::from_rotation_y(angle + PI / BARS as f32);
                bin.spawn((
                    Transform::from_rotation(half).with_translation(half * Vec3::Z * radius),
                    Visibility::Inherited,
                ))
                .with_children(|rims| {
                    rims.spawn((Shape::cuboid(&rim, WIRE), Finish::Metal));
                });
            }
        });
    }
}

impl Binned {
    /// Crumples up `slip` and tosses it into the bin.
    pub(crate) fn send(slip: Entity, commands: &mut Commands) {
        commands
            .entity(slip)
            .remove::<(FocusTarget, Clickable)>()
            .insert(Self::default());
    }

    fn toss(
        time: Res<Time>,
        bins: Query<&Transform, (With<Bin>, Without<Binned>)>,
        mut slips: Query<(Entity, &mut Binned, &mut Transform)>,
        mut commands: Commands,
    ) {
        let Some(bin) = bins.iter().next() else {
            return;
        };
        let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
        // Over the edge of the counter, then down into the bin.
        let edge = CounterLayout::new().front_edge(bin.translation.x)
            + Vec3::new(0.0, ARC / 2.0, PAST_EDGE);
        let into = bin.translation + Vec3::Y * BOTTOM * 4.0;
        for (entity, mut slip, mut transform) in &mut slips {
            let from = *slip.from.get_or_insert(*transform);
            slip.elapsed += time.delta_secs();
            // Crumpled up where it lies, then tossed.
            let crumpled = smooth((slip.elapsed / CRUMPLE_TIME).min(1.0));
            transform.scale = Vec3::splat(1.0 - (1.0 - CRUMPLED) * crumpled);
            let toss = ((slip.elapsed - CRUMPLE_TIME) / TOSS_TIME).clamp(0.0, 1.0);
            let fall = ((slip.elapsed - CRUMPLE_TIME - TOSS_TIME) / FALL_TIME).clamp(0.0, 1.0);
            let eased = smooth(toss);
            transform.translation = if fall > 0.0 {
                // Speeds up as it falls.
                edge.lerp(into.with_z(edge.z), fall * fall)
            } else {
                from.translation.lerp(edge, eased) + Vec3::Y * ARC * (PI * eased).sin()
            };
            transform.rotation = from.rotation * Quat::from_rotation_x(TAU * (toss + fall));
            if fall >= 1.0 {
                commands.entity(entity).despawn();
                commands.trigger(Play(Sound::PageSlide));
            }
        }
    }
}
