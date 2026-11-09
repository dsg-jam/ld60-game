use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_devices::Movement;
use ld60_game_audio::{Play, Sound};
use ld60_game_city::DroppedOff;
#[cfg(feature = "debug_port")]
use ld60_game_core::debug_port::{DebugMethod, DebugMethods};
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_core::state::AppState;

use ld60_game_player::focus::{Clickable, Focus};
use ld60_tick::Revision;

use super::movement;
use super::{Flyer, Trainer};

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

/// A movement that came in a parcel and waits to go on the front panel.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
struct Loose(Movement);

impl Parcel {
    pub(super) fn register(app: &mut App) {
        app.add_systems(Update, Self::send_first.run_if(in_state(AppState::Playing)))
            .add_observer(Loose::arrive)
            .add_observer(Loose::mount);
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
                        Loose(Movement::blank(Revision::A)),
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

impl Loose {
    /// Once on the counter, the movement can go on the panel.
    fn arrive(
        dropped: On<DroppedOff>,
        parcels: Query<&Children, With<Parcel>>,
        loose: Query<(), With<Loose>>,
        mut commands: Commands,
    ) {
        let Ok(children) = parcels.get(dropped.items) else {
            return;
        };
        for &child in children {
            if loose.contains(child) {
                commands.entity(child).insert(Clickable {
                    prompt: "click-mount-movement",
                });
            }
        }
    }

    /// Clicking a movement on the counter puts it on the front panel.
    fn mount(
        mut press: On<PointerPress>,
        state: Res<State<Focus>>,
        loose: Query<&Loose, With<Clickable>>,
        mut trainers: Query<&mut Trainer>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || *state.get() != Focus::Desk {
            return;
        }
        let Ok(Loose(movement)) = loose.get(press.entity) else {
            return;
        };
        let Ok(mut trainer) = trainers.single_mut() else {
            return;
        };
        press.propagate(false);
        trainer.mount(movement.clone());
        commands.entity(press.entity).despawn();
        commands.trigger(Play(Sound::PlugIn));
    }
}
