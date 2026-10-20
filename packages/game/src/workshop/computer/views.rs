use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    window::PrimaryWindow,
};

use super::pinboard::PinBoard;
use crate::locale::Message;
use crate::menu::AppState;
use crate::player::camera::{PlayerCamera, framing};
use crate::player::focus::{Aside, CloseUp, Focus, FocusTarget, View};
use crate::player::hints::{Hint, Hints, Icon, Input};
use crate::workshop::run::Run;

/// How far the wide view looks down at the keyboard, in radians.
const WIDE_PITCH: f32 = 0.3;
/// Free space around the screen and the keyboard in the wide view, as a
/// fraction of the window.
const WIDE_MARGIN: f32 = 0.03;
/// Free space around the glass in the close view, as a fraction of the
/// window.
const CLOSE_MARGIN: f32 = 0.05;
const ZOOM_KEY: KeyCode = KeyCode::F1;
/// Held with the wheel to zoom, like in most programs.
pub(super) const ZOOM_MODIFIERS: [KeyCode; 2] = [KeyCode::ControlLeft, KeyCode::ControlRight];
/// How far a trackpad has to scroll to zoom.
const ZOOM_PIXELS: f32 = 40.0;
/// Also the footer command of the pinned sheets.
const GLANCE_KEY: KeyCode = KeyCode::F12;
/// Holding the glance key longer than this, in seconds, looks back on
/// release.
const HOLD_TIME: f32 = 0.3;
/// How close to the left edge of the window the pointer pushes, in logical
/// pixels.
const EDGE: f32 = 4.0;
/// How long the pointer has to rest against the edge to glance, in seconds,
/// so a quick pass doesn't.
const DWELL_TIME: f32 = 0.2;
/// The pointer moving this far into the window, as a fraction of its width,
/// looks back from a glance with the pointer.
const RETURN: f32 = 0.1;

pub(super) struct ViewsPlugin;

impl Plugin for ViewsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Focus::Computer), Views::restore_zoom)
            .add_systems(
                Update,
                (
                    Views::frame,
                    (Views::zoom_on_input, Glance::update).run_if(in_state(AppState::Playing)),
                    Views::show_hints,
                )
                    .chain(),
            );
    }
}

/// The views of the computer. Its [`FocusTarget`] looks at the screen and
/// the keyboard, its [`CloseUp`] at the screen alone and its [`Aside`] at
/// the pinned sheets. Each frames its subject in the window.
#[derive(Component, Debug)]
pub(super) struct Views {
    screen: [Vec3; 4],
    desk: Vec<Vec3>,
    /// The zoom the player picked last, for the next time at the computer.
    close: bool,
}

impl Views {
    /// `screen` are the corners of the glass and `desk` what the wide view
    /// shows along with them.
    pub(super) fn new(screen: [Vec3; 4], desk: impl IntoIterator<Item = Vec3>) -> Self {
        Self {
            screen,
            desk: screen.into_iter().chain(desk).collect(),
            close: false,
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
    pub(super) fn initial(&self, board: &PinBoard) -> (View, CloseUp, Aside) {
        let perspective = PerspectiveProjection::default();
        (
            View::Local(self.wide(&perspective)),
            CloseUp::new(self.close(&perspective)),
            Aside::new(board.view(0, &perspective)),
        )
    }

    /// Follows the shape of the window and the number of pinned sheets.
    fn frame(
        camera: Single<&Projection, With<PlayerCamera>>,
        run: Res<Run>,
        mut computers: Query<(
            &Views,
            &PinBoard,
            &mut FocusTarget,
            &mut CloseUp,
            &mut Aside,
        )>,
    ) {
        let Projection::Perspective(perspective) = *camera else {
            return;
        };
        for (views, board, mut target, mut close_up, mut aside) in &mut computers {
            let wide = views.wide(perspective);
            if !matches!(target.view, View::Local(view) if view == wide) {
                target.view = View::Local(wide);
            }
            let close = views.close(perspective);
            if close_up.view != close {
                close_up.view = close;
            }
            let pins = board.view(run.pins().sheets().len(), perspective);
            if aside.view != pins {
                aside.view = pins;
            }
        }
    }

    fn restore_zoom(mut computers: Query<(&Views, &mut CloseUp)>) {
        for (views, mut close_up) in &mut computers {
            close_up.open = views.close;
        }
    }

    /// The zoom key switches between the wide and the close view. The wheel
    /// with Ctrl held zooms in and out, since the wheel alone scrolls the
    /// editor.
    fn zoom_on_input(
        keys: Res<ButtonInput<KeyCode>>,
        mut wheel: MessageReader<MouseWheel>,
        state: Res<State<Focus>>,
        mut computers: Query<(&mut Views, &mut CloseUp, &Aside)>,
        mut swipe: Local<f32>,
    ) {
        let held = keys.any_pressed(ZOOM_MODIFIERS);
        let mut zoom = None;
        for event in wheel.read() {
            if !held {
                continue;
            }
            match event.unit {
                MouseScrollUnit::Line => zoom = Some(event.y > 0.0),
                MouseScrollUnit::Pixel => {
                    if *swipe * event.y < 0.0 {
                        *swipe = 0.0;
                    }
                    *swipe += event.y;
                    if swipe.abs() >= ZOOM_PIXELS {
                        zoom = Some(*swipe > 0.0);
                        *swipe = 0.0;
                    }
                }
            }
        }
        if !held {
            *swipe = 0.0;
        }
        if *state.get() != Focus::Computer {
            return;
        }
        for (mut views, mut close_up, aside) in &mut computers {
            if aside.open {
                continue;
            }
            let close = if keys.just_pressed(ZOOM_KEY) {
                !close_up.open
            } else if let Some(close) = zoom {
                close
            } else {
                continue;
            };
            views.close = close;
            if close_up.open != close {
                close_up.open = close;
            }
        }
    }

    fn show_hints(
        state: Res<State<Focus>>,
        run: Res<Run>,
        mut computers: Query<(&Aside, &CloseUp, &Glance, &mut Hints)>,
    ) {
        if *state.get() != Focus::Computer {
            return;
        }
        for (aside, close_up, glance, mut hints) in &mut computers {
            let mut shown = Vec::new();
            if aside.open {
                let mut inputs = vec![Input::Key("F12")];
                if glance.gesture == Some(Gesture::Edge) {
                    inputs.push(Input::Icon(Icon::Sideways));
                }
                shown.push(Hint {
                    inputs,
                    action: Message::new("hint-look-screen"),
                });
            } else {
                shown.push(Hint {
                    inputs: vec![Input::Key("F1"), Input::Chord("Ctrl", Icon::Wheel)],
                    action: Message::new(if close_up.open {
                        "hint-zoom-out"
                    } else {
                        "hint-zoom-in"
                    }),
                });
                if !run.pins().is_empty() {
                    shown.push(Hint {
                        inputs: vec![Input::Key("F12"), Input::Icon(Icon::Sideways)],
                        action: Message::new("hint-look-pins"),
                    });
                }
            }
            hints.set_if_neq(Hints(shown));
        }
    }
}

/// Turns the head to the pinned sheets while at the computer, with the
/// glance key or by pushing the pointer against the left edge of the window.
///
/// A tap of the key keeps looking until the next press, holding it looks
/// back on release. A glance with the pointer looks back once the pointer
/// moves away from the edge.
#[derive(Component, Debug, Default, PartialEq)]
pub(super) struct Glance {
    /// How the open glance started, so the same gesture ends it.
    gesture: Option<Gesture>,
    /// Seconds the pointer has rested against the edge.
    dwell: f32,
    /// Whether the pointer left the edge since the last glance, so resting
    /// there again glances again.
    armed: bool,
    /// The last place of the pointer across the window, to tell where it
    /// left the window.
    last_x: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Gesture {
    Key { since: f32 },
    Edge,
}

/// Where the pointer is, from the left edge of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pointer {
    /// Against the edge, or gone from the window over it.
    AtEdge,
    Near,
    Away,
}

impl Glance {
    fn update(
        time: Res<Time>,
        keys: Res<ButtonInput<KeyCode>>,
        state: Res<State<Focus>>,
        run: Res<Run>,
        window: Single<&Window, With<PrimaryWindow>>,
        mut computers: Query<(&mut Glance, &mut Aside)>,
    ) {
        let usable = *state.get() == Focus::Computer && !run.pins().is_empty();
        for (mut glance, mut aside) in &mut computers {
            if !usable {
                glance.set_if_neq(Glance::default());
                continue;
            }
            let pointer = glance.locate(&window);
            let mut open = aside.open;
            glance.step(
                &mut open,
                pointer,
                (
                    keys.just_pressed(GLANCE_KEY),
                    keys.just_released(GLANCE_KEY),
                ),
                time.elapsed_secs(),
                time.delta_secs(),
            );
            if aside.open != open {
                aside.open = open;
            }
        }
    }

    fn locate(&mut self, window: &Window) -> Pointer {
        let width = window.width();
        match window.cursor_position() {
            Some(position) => {
                self.last_x = Some(position.x);
                if position.x <= EDGE {
                    Pointer::AtEdge
                } else if position.x < width * RETURN {
                    Pointer::Near
                } else {
                    Pointer::Away
                }
            }
            None if self.last_x.is_some_and(|x| x < width * RETURN) => Pointer::AtEdge,
            None => Pointer::Near,
        }
    }

    /// Opens or closes the glance in `open`, with the glance key just
    /// `(pressed, released)`, at `now` seconds, `dt` after the last step.
    fn step(&mut self, open: &mut bool, pointer: Pointer, key: (bool, bool), now: f32, dt: f32) {
        if !*open {
            self.gesture = None;
        }
        let (pressed, released) = key;
        if pressed {
            *open = !*open;
            self.gesture = open.then_some(Gesture::Key { since: now });
        }
        let held = matches!(self.gesture, Some(Gesture::Key { since }) if now - since >= HOLD_TIME);
        let left_edge = self.gesture == Some(Gesture::Edge) && pointer == Pointer::Away;
        if (released && held) || left_edge {
            *open = false;
            self.gesture = None;
        }
        if *open {
            self.armed = false;
            self.dwell = 0.0;
        } else if pointer != Pointer::AtEdge {
            self.armed = true;
            self.dwell = 0.0;
        } else if self.armed {
            self.dwell += dt;
            if self.dwell >= DWELL_TIME {
                *open = true;
                self.gesture = Some(Gesture::Edge);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::Locale;

    const DT: f32 = 1.0 / 60.0;

    /// Steps `glance` for `seconds` with the pointer at `pointer`, starting
    /// at `start`, and returns the time after.
    fn rest(
        glance: &mut Glance,
        open: &mut bool,
        pointer: Pointer,
        start: f32,
        seconds: f32,
    ) -> f32 {
        let mut now = start;
        while now < start + seconds {
            now += DT;
            glance.step(open, pointer, (false, false), now, DT);
        }
        now
    }

    #[test]
    fn resting_at_the_edge_glances_until_the_pointer_leaves() {
        let mut glance = Glance::default();
        let mut open = false;
        let now = rest(&mut glance, &mut open, Pointer::Away, 0.0, 0.1);
        let now = rest(
            &mut glance,
            &mut open,
            Pointer::AtEdge,
            now,
            DWELL_TIME / 2.0,
        );
        assert!(!open);
        let now = rest(&mut glance, &mut open, Pointer::AtEdge, now, DWELL_TIME);
        assert!(open);
        let now = rest(&mut glance, &mut open, Pointer::Near, now, 1.0);
        assert!(open);
        rest(&mut glance, &mut open, Pointer::Away, now, DT);
        assert!(!open);
    }

    #[test]
    fn going_back_at_the_edge_needs_the_pointer_to_leave_first() {
        let mut glance = Glance::default();
        let mut open = false;
        let now = rest(&mut glance, &mut open, Pointer::AtEdge, 0.0, 1.0);
        assert!(!open, "the pointer was there before");
        let now = rest(&mut glance, &mut open, Pointer::Near, now, DT);
        let now = rest(&mut glance, &mut open, Pointer::AtEdge, now, 1.0);
        assert!(open);
        open = false;
        let now = rest(&mut glance, &mut open, Pointer::AtEdge, now, 1.0);
        assert!(!open);
        let now = rest(&mut glance, &mut open, Pointer::Near, now, DT);
        rest(&mut glance, &mut open, Pointer::AtEdge, now, 1.0);
        assert!(open);
    }

    #[test]
    fn tapping_the_key_toggles_and_holding_it_looks_back_on_release() {
        let mut glance = Glance::default();
        let mut open = false;
        glance.step(&mut open, Pointer::Away, (true, true), 0.0, DT);
        assert!(open);
        let now = rest(&mut glance, &mut open, Pointer::Away, 0.0, 2.0);
        assert!(open);
        glance.step(&mut open, Pointer::Away, (true, false), now, DT);
        assert!(!open);
        glance.step(&mut open, Pointer::Away, (false, true), now + 0.1, DT);
        assert!(!open);

        glance.step(&mut open, Pointer::Away, (true, false), 10.0, DT);
        assert!(open);
        glance.step(
            &mut open,
            Pointer::Away,
            (false, true),
            10.0 + HOLD_TIME / 2.0,
            DT,
        );
        assert!(open, "a tap");
        glance.step(&mut open, Pointer::Away, (true, false), 20.0, DT);
        assert!(!open);
        glance.step(&mut open, Pointer::Away, (true, false), 30.0, DT);
        glance.step(
            &mut open,
            Pointer::Away,
            (false, true),
            30.0 + 2.0 * HOLD_TIME,
            DT,
        );
        assert!(!open, "a hold");
    }

    #[test]
    fn the_key_ends_a_glance_with_the_pointer() {
        let mut glance = Glance::default();
        let mut open = false;
        let now = rest(&mut glance, &mut open, Pointer::Near, 0.0, DT);
        let now = rest(&mut glance, &mut open, Pointer::AtEdge, now, 1.0);
        assert!(open);
        glance.step(&mut open, Pointer::AtEdge, (true, true), now, DT);
        assert!(!open);
        rest(&mut glance, &mut open, Pointer::AtEdge, now, 1.0);
        assert!(!open);
    }

    #[test]
    fn every_hint_formats() {
        Locale::on_disk().assert_formats(
            [
                "hint-zoom-in",
                "hint-zoom-out",
                "hint-look-pins",
                "hint-look-screen",
            ]
            .map(Message::new),
        );
    }
}
