use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_game_audio::{Play, Sound};
use ld60_game_player::cursor::WantsFinger;
use ld60_game_player::focus::Focused;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

/// The threaded bushing the lever sticks out of.
const BUSHING: Vec2 = Vec2::new(0.0035, 0.004);
const NUT: Vec2 = Vec2::new(0.0055, 0.0015);
/// The bat-shaped lever: its thickness and length.
const LEVER: Vec2 = Vec2::new(0.0022, 0.011);
/// How far the lever leans over either way.
const THROW: f32 = 0.4;
/// Seconds the lever takes to snap over.
const SNAP_TIME: f32 = 0.04;

const CHROME: Color = Color::srgb_u8(0xc4, 0xc6, 0xc8);

pub struct TogglePlugin;

impl Plugin for TogglePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Toggle::show)
            .add_observer(Toggle::on_add)
            .add_observer(Toggle::flip);
    }
}

/// A small toggle switch with a chrome lever. It stands up along +Y from
/// the panel at the origin. Up leans the lever towards -Z. A click flips it
/// while the thing it is on is focused.
#[derive(Component, Debug)]
#[require(Transform, Visibility, WantsFinger)]
pub struct Toggle {
    pub up: bool,
}

/// The lever of a [`Toggle`], turning about the top of the bushing.
#[derive(Component, Debug)]
struct Lever;

impl Toggle {
    fn on_add(add: On<Add<Toggle>>, toggles: Query<&Toggle>, mut commands: Commands) {
        let Ok(toggle) = toggles.get(add.entity) else {
            return;
        };
        let bushing = Block::new(Vec3::new(2.0 * BUSHING.x, BUSHING.y, 2.0 * BUSHING.x));
        let nut = Block::new(Vec3::new(2.0 * NUT.x, NUT.y, 2.0 * NUT.x));
        let lever = Block::new(Vec3::new(LEVER.x, LEVER.y, LEVER.x));
        let up = toggle.up;
        commands.entity(add.entity).with_children(|parent| {
            parent.spawn((Shape::cylinder(&nut, CHROME), Finish::Plated));
            parent.spawn((Shape::cylinder(&bushing, CHROME), Finish::Plated));
            parent
                .spawn((
                    Lever,
                    Transform::from_xyz(0.0, BUSHING.y, 0.0).with_rotation(Self::lean(up)),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    pivot.spawn((Shape::cylinder(&lever, CHROME), Finish::Plated));
                });
        });
    }

    fn lean(up: bool) -> Quat {
        Quat::from_rotation_x(if up { -THROW } else { THROW })
    }

    fn flip(
        mut press: On<PointerPress>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        mut toggles: Query<&mut Toggle>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || !focused.holds(press.entity, &parents) {
            return;
        }
        let Ok(mut toggle) = toggles.get_mut(press.entity) else {
            return;
        };
        press.propagate(false);
        toggle.up = !toggle.up;
        commands.trigger(Play(if toggle.up {
            Sound::Latch
        } else {
            Sound::Unlatch
        }));
    }

    fn show(
        time: Res<Time>,
        toggles: Query<(&Toggle, &Children)>,
        mut levers: Query<&mut Transform, With<Lever>>,
    ) {
        let step = 2.0 * THROW * time.delta_secs() / SNAP_TIME;
        for (toggle, children) in &toggles {
            for &child in children {
                if let Ok(mut transform) = levers.get_mut(child) {
                    let goal = Self::lean(toggle.up);
                    let angle = transform.rotation.angle_between(goal);
                    if angle > 0.0 {
                        let rotation = transform.rotation.slerp(goal, (step / angle).min(1.0));
                        transform.rotation = rotation;
                    }
                }
            }
        }
    }
}
