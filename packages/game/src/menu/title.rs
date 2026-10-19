use std::f32::consts::TAU;

use bevy::prelude::*;

use super::list::Action;
use super::{AppState, Column, Started};
use crate::locale::Message;
use crate::player::camera::PlayerCamera;

/// The camera looks at the shop from across the street.
const EYE: Vec3 = Vec3::new(2.2, 1.5, -5.0);
const AT: Vec3 = Vec3::new(-0.2, 1.4, 0.0);
/// How far the camera sways to the side and up, in meters.
const SWAY: Vec2 = Vec2::new(0.35, 0.06);
/// Seconds for one sway to the side and back.
const SWAY_TIME: f32 = 40.0;
/// How quickly the camera glides out to the street after playing.
const DECAY_RATE: f32 = 1.5;

/// The entries of the title screen before the first start.
pub(super) const START_ENTRIES: &[(Action, &str)] = &[
    (Action::Start, "menu-start"),
    (Action::Settings, "menu-settings"),
    (Action::Credits, "menu-credits"),
    #[cfg(not(target_arch = "wasm32"))]
    (Action::Quit, "menu-quit"),
];
/// The entries of the title screen once the player has started, or when a
/// saved run waits.
pub(super) const CONTINUE_ENTRIES: &[(Action, &str)] = &[
    (Action::Start, "menu-continue"),
    (Action::Restart, "menu-restart"),
    (Action::Settings, "menu-settings"),
    (Action::Credits, "menu-credits"),
    #[cfg(not(target_arch = "wasm32"))]
    (Action::Quit, "menu-quit"),
];

pub(super) struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Title), spawn)
            .add_systems(Update, drift.run_if(in_state(AppState::Title)));
    }
}

fn spawn(mut commands: Commands) {
    commands.spawn(Column::screen(AppState::Title, Message::new("menu-game")));
}

/// Sways the camera slowly in front of the shop. Before the first start it
/// is there right away, later it glides out from the seat.
fn drift(
    time: Res<Time>,
    started: Res<Started>,
    mut camera: Single<&mut Transform, With<PlayerCamera>>,
) {
    let turn = time.elapsed_secs() / SWAY_TIME * TAU;
    let eye = EYE + Vec3::new(SWAY.x * turn.sin(), SWAY.y * (turn * 1.7).sin(), 0.0);
    let goal = Transform::from_translation(eye).looking_at(AT, Vec3::Y);
    if !started.0 {
        **camera = goal;
        return;
    }
    let dt = time.delta_secs();
    camera
        .translation
        .smooth_nudge(&goal.translation, DECAY_RATE, dt);
    camera.rotation.smooth_nudge(&goal.rotation, DECAY_RATE, dt);
}
