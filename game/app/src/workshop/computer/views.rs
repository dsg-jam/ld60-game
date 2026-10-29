use std::iter;

use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_game_core::player::camera::{PlayerCamera, framing};
use ld60_game_core::player::focus::{Aside, Clickable, Focus, FocusTarget, View};
use ld60_game_core::player::free_look::MODIFIERS;
use ld60_game_core::player::hints::{Hint, Hints, Icon, Input};
use ld60_game_core::player::lens;
use ld60_game_core::player::zoom::Zoom;
use ld60_game_core::state::AppState;
use ld60_game_manuals::Pins;
use ld60_locale::Message;

use super::pinboard::PinBoard;

/// How far the wide view looks down at the keyboard, in radians.
const WIDE_PITCH: f32 = 0.3;
/// Free space around the screen and the keyboard in the wide view, as a
/// fraction of the window.
const WIDE_MARGIN: f32 = 0.03;
/// Free space around the glass in the close view, as a fraction of the
/// window.
const CLOSE_MARGIN: f32 = 0.05;
/// Steps of the [`Zoom`] from the wide view to the close one.
const ZOOM_STEPS: u8 = 2;
/// With Ctrl, turns to the pinned sheets and back. The editor has no use
/// for Ctrl+P, and P also pins pages in the manuals file.
const PINS_KEY: KeyCode = KeyCode::KeyP;

pub(super) struct ViewsPlugin;

impl Plugin for ViewsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Views::frame,
                Views::look_aside_on_key.run_if(in_state(AppState::Playing)),
                (Views::show_hints, BackToScreen::arm),
            )
                .chain(),
        )
        .add_observer(BackToScreen::look_back_on_press);
    }
}

/// The views of the computer. Its [`FocusTarget`] looks at the screen and
/// the keyboard, the end of its [`Zoom`] at the screen alone and its
/// [`Aside`] at the pinned sheets. Each frames its subject in the window.
#[derive(Component, Debug)]
pub(super) struct Views {
    screen: [Vec3; 4],
    desk: Vec<Vec3>,
}

/// The monitor, which a click looks back at from the pinned sheets.
#[derive(Component, Debug)]
pub(super) struct BackToScreen;

impl Views {
    /// `screen` are the corners of the glass and `desk` what the wide view
    /// shows along with them.
    pub(super) fn new(screen: [Vec3; 4], desk: impl IntoIterator<Item = Vec3>) -> Self {
        Self {
            screen,
            desk: screen.into_iter().chain(desk).collect(),
        }
    }

    fn wide(&self, perspective: &PerspectiveProjection) -> Transform {
        framing(
            Quat::from_rotation_x(-WIDE_PITCH),
            self.desk.iter().copied(),
            perspective,
            WIDE_MARGIN,
        )
    }

    fn close(&self, perspective: &PerspectiveProjection) -> Transform {
        framing(Quat::IDENTITY, self.screen, perspective, CLOSE_MARGIN)
    }

    /// The views for a window of the default shape, until the camera has
    /// one.
    pub(super) fn initial(&self, board: &PinBoard) -> (View, Zoom, Aside) {
        let perspective = PerspectiveProjection::default();
        (
            View::Local(self.wide(&perspective)),
            Zoom::to(View::Local(self.close(&perspective)), ZOOM_STEPS),
            Aside::new(board.view(0, &perspective)),
        )
    }

    /// Follows the shape of the window and the number of pinned sheets.
    fn frame(
        camera: Single<&Projection, With<PlayerCamera>>,
        pins: Res<Pins>,
        mut computers: Query<(&Views, &PinBoard, &mut FocusTarget, &mut Zoom, &mut Aside)>,
    ) {
        let Projection::Perspective(perspective) = *camera else {
            return;
        };
        // The views frame their subject without the zoom of the lens.
        let perspective = &PerspectiveProjection {
            fov: lens::FOV,
            ..perspective.clone()
        };
        for (views, board, mut target, mut zoom, mut aside) in &mut computers {
            let wide = views.wide(perspective);
            if !matches!(target.view, View::Local(view) if view == wide) {
                target.view = View::Local(wide);
            }
            let close = View::Local(views.close(perspective));
            if zoom.close != close {
                zoom.close = close;
            }
            let pins = board.view(pins.used(), perspective);
            if aside.view != pins {
                aside.view = pins;
            }
        }
    }

    /// Ctrl+P turns to the pinned sheets and back. It works whether the
    /// editor has the keyboard or not.
    fn look_aside_on_key(
        keys: Res<ButtonInput<KeyCode>>,
        state: Res<State<Focus>>,
        pins: Res<Pins>,
        mut computers: Query<&mut Aside, With<Views>>,
    ) {
        if *state.get() != Focus::Computer
            || !keys.any_pressed(MODIFIERS)
            || !keys.just_pressed(PINS_KEY)
        {
            return;
        }
        for mut aside in &mut computers {
            if aside.open || !pins.is_empty() {
                aside.open = !aside.open;
            }
        }
    }

    fn show_hints(
        state: Res<State<Focus>>,
        pins: Res<Pins>,
        mut computers: Query<(&Aside, &mut Hints), With<Views>>,
    ) {
        if *state.get() != Focus::Computer {
            return;
        }
        for (aside, mut hints) in &mut computers {
            let look = |action| Hint {
                inputs: vec![Input::Shortcut("Ctrl", "P")],
                action: Message::new(action),
            };
            let shown = if aside.open {
                vec![look("hint-look-screen")]
            } else {
                let mut shown = vec![Hint {
                    inputs: vec![Input::Icon(Icon::Wheel)],
                    action: Message::new("hint-scroll"),
                }];
                if !pins.is_empty() {
                    shown.push(look("hint-look-pins"));
                }
                shown
            };
            hints.set_if_neq(Hints(shown));
        }
    }
}

impl BackToScreen {
    /// The monitor says what a click does while the player looks at the
    /// pinned sheets.
    fn arm(
        state: Res<State<Focus>>,
        computers: Query<&Aside, With<Views>>,
        monitors: Query<(Entity, Has<Clickable>), With<BackToScreen>>,
        mut commands: Commands,
    ) {
        let aside = *state.get() == Focus::Computer && computers.iter().any(|aside| aside.open);
        for (monitor, clickable) in &monitors {
            match (aside, clickable) {
                (true, false) => {
                    commands.entity(monitor).insert(Clickable {
                        prompt: "click-look-screen",
                    });
                }
                (false, true) => {
                    commands.entity(monitor).remove::<Clickable>();
                }
                _ => {}
            }
        }
    }

    /// The press goes on to the screen, so a click there also places the
    /// caret.
    fn look_back_on_press(
        press: On<PointerPress>,
        state: Res<State<Focus>>,
        parents: Query<&ChildOf>,
        monitors: Query<(), With<BackToScreen>>,
        mut computers: Query<&mut Aside, With<Views>>,
    ) {
        if press.button != PointerButton::Primary || *state.get() != Focus::Computer {
            return;
        }
        let mut path = iter::once(press.entity).chain(parents.iter_ancestors(press.entity));
        if !path.any(|entity| monitors.contains(entity)) {
            return;
        }
        let Some(computer) = path.find(|&ancestor| computers.contains(ancestor)) else {
            return;
        };
        let Ok(mut aside) = computers.get_mut(computer) else {
            return;
        };
        if aside.open {
            aside.open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_game_manuals::Sheet;
    use ld60_locale::Locale;

    use super::*;

    #[test]
    fn every_hint_formats() {
        Locale::on_disk().assert_formats(
            [
                "hint-look-pins",
                "hint-look-screen",
                "hint-scroll",
                "click-look-pins",
                "click-look-screen",
            ]
            .map(Message::new),
        );
    }

    #[test]
    fn ctrl_p_turns_to_the_pins_and_back() {
        let mut world = World::new();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.insert_resource(State::new(Focus::Computer));
        let mut pins = Pins::default();
        pins.pin(Sheet::example());
        world.insert_resource(pins);
        let computer = world
            .spawn((
                Views::new([Vec3::ZERO; 4], []),
                Aside::new(Transform::IDENTITY),
            ))
            .id();
        let press = |world: &mut World, keys: &[KeyCode]| {
            let mut input = world.resource_mut::<ButtonInput<KeyCode>>();
            input.reset_all();
            for &key in keys {
                input.press(key);
            }
            world.run_system_cached(Views::look_aside_on_key).unwrap();
            world.get::<Aside>(computer).unwrap().open
        };
        assert!(!press(&mut world, &[PINS_KEY]), "P alone types");
        assert!(press(&mut world, &[KeyCode::ControlLeft, PINS_KEY]));
        assert!(!press(&mut world, &[KeyCode::ControlRight, PINS_KEY]));
        world.resource_mut::<Pins>().unpin(Sheet::example());
        assert!(!press(&mut world, &[KeyCode::ControlLeft, PINS_KEY]));
    }
}
