use bevy::prelude::*;
#[cfg(feature = "debug_port")]
use ld60_game_core::debug_port::{DebugMethod, DebugMethods};
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_core::state::AppState;

use super::Flyer;
use super::movement;

/// Where a visitor puts things down: across the counter from the player,
/// right of the stamp, this far in from the street edge. The visitor
/// stands between the till and the stamp, this far in front of the
/// counter, where the player sees it.
const DROP_X: f32 = 0.3;
const DROP_IN: f32 = 0.12;
const STAND_X: f32 = 0.08;
const STAND_BACK: f32 = 0.36;
/// The movement lies on the flyer, this far off its middle.
const MOVEMENT_ON_FLYER: Vec3 = Vec3::new(0.035, 0.0005, 0.0);

/// What a customer brings for a watch job: the flyer with a bare movement
/// lying on it.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Parcel;

impl Parcel {
    pub(super) fn register(app: &mut App) {
        app.add_systems(Update, Self::send_first.run_if(in_state(AppState::Playing)));
        #[cfg(feature = "debug_port")]
        app.add_debug_method(
            DebugMethod::new(
                "ld60.visit",
                "Someone walking by brings a watch movement and its flyer to the counter.",
            ),
            Self::debug_visit,
        );
    }

    /// For now, someone brings a watch once the player is in the shop.
    fn send_first(mut sent: Local<bool>, mut commands: Commands) {
        if !*sent {
            *sent = true;
            Self::send(&mut commands, 1);
        }
    }

    /// Asks someone walking by to bring a parcel to the counter.
    fn send(commands: &mut Commands, seed: u64) {
        let items = Self::spawn(commands, seed);
        let (stand, drop) = Self::drop_spot();
        commands.trigger(ld60_game_city::DropOff {
            items,
            stand,
            drop,
            point_at: drop.translation,
        });
    }

    /// Spawns a parcel out of sight, for a visitor to carry.
    fn spawn(commands: &mut Commands, seed: u64) -> Entity {
        commands
            .spawn((
                Name::new("parcel"),
                Parcel,
                Transform::from_xyz(0.0, -10.0, 0.0),
            ))
            .with_children(|parcel| {
                parcel.spawn((Name::new("flyer"), Flyer { seed }));
                parcel
                    .spawn((
                        Transform::from_translation(MOVEMENT_ON_FLYER).with_scale(Vec3::splat(0.9)),
                        Visibility::default(),
                    ))
                    .with_children(movement::spawn);
            })
            .id()
    }

    /// Where a visitor stands at the counter, and where it puts a parcel
    /// down.
    fn drop_spot() -> (Vec2, Transform) {
        let top = CounterLayout::new().top;
        let stand = Vec2::new(STAND_X, top.min().z - STAND_BACK);
        (
            stand,
            Transform::from_xyz(DROP_X, top.max().y, top.min().z + DROP_IN),
        )
    }

    #[cfg(feature = "debug_port")]
    fn debug_visit(
        In(_): In<Option<serde_json::Value>>,
        mut commands: Commands,
    ) -> bevy::remote::BrpResult {
        Self::send(&mut commands, 1);
        Ok(serde_json::Value::Null)
    }
}
