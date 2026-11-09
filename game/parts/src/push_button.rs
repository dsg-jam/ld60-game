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

/// The square cap: its width and height above the panel at rest.
const CAP: Vec2 = Vec2::new(0.009, 0.005);
const BEZEL: Vec2 = Vec2::new(0.012, 0.0015);
/// How far the cap goes down while pressed.
const TRAVEL: f32 = 0.002;

const BEZEL_COLOR: Color = Color::srgb_u8(0x20, 0x20, 0x22);

pub struct PushButtonPlugin;

impl Plugin for PushButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, PushButton::release)
            .add_observer(PushButton::on_add)
            .add_observer(PushButton::press);
    }
}

/// A square push button that springs back up. It stands up along +Y from
/// the panel at the origin, and triggers [`Pushed`] when pressed while the
/// thing it is on is focused.
#[derive(Component, Debug)]
#[require(Transform, Visibility, WantsFinger)]
pub struct PushButton {
    pub color: Color,
    held: bool,
}

/// A [`PushButton`] went down.
#[derive(EntityEvent, Debug)]
pub struct Pushed {
    pub entity: Entity,
}

/// The moving cap of a [`PushButton`].
#[derive(Component, Debug)]
struct Cap;

impl PushButton {
    pub fn new(color: Color) -> Self {
        Self { color, held: false }
    }

    fn on_add(add: On<Add<PushButton>>, buttons: Query<&PushButton>, mut commands: Commands) {
        let Ok(button) = buttons.get(add.entity) else {
            return;
        };
        let bezel = Block::new(Vec3::new(BEZEL.x, BEZEL.y, BEZEL.x));
        let cap = Block::new(Vec3::new(CAP.x, CAP.y, CAP.x));
        let color = button.color;
        commands.entity(add.entity).with_children(|parent| {
            parent.spawn((Shape::cuboid(&bezel, BEZEL_COLOR), Finish::Plastic));
            parent
                .spawn((Cap, Transform::default(), Visibility::default()))
                .with_children(|moving| {
                    moving.spawn((Shape::cuboid(&cap, color), Finish::Plastic));
                });
        });
    }

    fn press(
        mut press: On<PointerPress>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        mut buttons: Query<(&mut PushButton, &Children)>,
        mut caps: Query<&mut Transform, With<Cap>>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || !focused.holds(press.entity, &parents) {
            return;
        }
        let Ok((mut button, children)) = buttons.get_mut(press.entity) else {
            return;
        };
        press.propagate(false);
        button.held = true;
        Self::move_cap(children, &mut caps, true);
        commands.trigger(Play(Sound::KeyDown));
        commands.trigger(Pushed {
            entity: press.entity,
        });
    }

    /// Lets every held button spring back when the mouse button goes up.
    fn release(
        mouse: Res<ButtonInput<MouseButton>>,
        mut buttons: Query<(&mut PushButton, &Children)>,
        mut caps: Query<&mut Transform, With<Cap>>,
        mut commands: Commands,
    ) {
        if !mouse.just_released(MouseButton::Left) {
            return;
        }
        for (mut button, children) in &mut buttons {
            if button.held {
                button.held = false;
                Self::move_cap(children, &mut caps, false);
                commands.trigger(Play(Sound::KeyUp));
            }
        }
    }

    fn move_cap(children: &Children, caps: &mut Query<&mut Transform, With<Cap>>, down: bool) {
        for &child in children {
            if let Ok(mut transform) = caps.get_mut(child) {
                transform.translation.y = if down { -TRAVEL } else { 0.0 };
            }
        }
    }
}
