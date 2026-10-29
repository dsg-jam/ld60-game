use std::time::Duration;

use bevy::{
    ecs::message::{MessageCursor, MessageUpdateSystems},
    input::{ButtonState, mouse::MouseButtonInput},
    picking::PickingSystems,
    prelude::*,
    time::TimeSystems,
    window::WindowEvent,
};

/// How long after the window comes back a press still counts as the click
/// that brought it back.
const GRACE: Duration = Duration::from_millis(300);

pub(super) struct RefocusPlugin;

impl Plugin for RefocusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RefocusClick>().add_systems(
            First,
            RefocusClick::drop_click
                .after(MessageUpdateSystems)
                .after(TimeSystems)
                .before(PickingSystems::Input),
        );
    }
}

/// Drops the click that brings the window back into focus, before picking,
/// the screens or [`ButtonInput<MouseButton>`] see it.
///
/// Depending on the platform, the press comes before or after the window
/// gains focus. Its release is dropped with it.
#[derive(Resource, Default, Debug)]
struct RefocusClick {
    armed: Armed,
    /// Buttons whose press was dropped and whose release is still to come.
    swallowed: Vec<MouseButton>,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
enum Armed {
    #[default]
    No,
    /// The window lost focus and no press came since.
    Unfocused,
    /// The window came back and the next press until then is dropped.
    Until(Duration),
    /// A press came while the window was out of focus and was dropped.
    Taken,
}

impl RefocusClick {
    /// Follows `event` at `now`, and returns whether it gets through.
    fn passes(&mut self, event: &WindowEvent, now: Duration) -> bool {
        match event {
            WindowEvent::WindowFocused(focused) => {
                self.armed = match (focused.focused, self.armed) {
                    (false, _) => Armed::Unfocused,
                    (true, Armed::Taken) => Armed::No,
                    (true, _) => Armed::Until(now + GRACE),
                };
                true
            }
            WindowEvent::MouseButtonInput(input) => match input.state {
                ButtonState::Pressed => {
                    let (drop, armed) = match self.armed {
                        Armed::Unfocused => (true, Armed::Taken),
                        Armed::Until(until) if now <= until => (true, Armed::No),
                        Armed::Taken => (false, Armed::Taken),
                        _ => (false, Armed::No),
                    };
                    self.armed = armed;
                    if drop {
                        self.swallowed.push(input.button);
                    }
                    !drop
                }
                ButtonState::Released => {
                    let Some(index) = self.swallowed.iter().position(|&b| b == input.button) else {
                        return true;
                    };
                    self.swallowed.swap_remove(index);
                    false
                }
            },
            _ => true,
        }
    }

    fn drop_click(
        time: Res<Time<Real>>,
        mut refocus: ResMut<RefocusClick>,
        mut window_events: ResMut<Messages<WindowEvent>>,
        mut buttons: ResMut<Messages<MouseButtonInput>>,
        mut events_read: Local<MessageCursor<WindowEvent>>,
        mut buttons_read: Local<MessageCursor<MouseButtonInput>>,
    ) {
        let now = time.elapsed();
        let fresh: Vec<WindowEvent> = events_read.read(&window_events).cloned().collect();
        let mut dropped = Vec::new();
        let kept: Vec<WindowEvent> = fresh
            .into_iter()
            .filter(|event| {
                let passes = refocus.passes(event, now);
                if !passes && let WindowEvent::MouseButtonInput(input) = event {
                    dropped.push(*input);
                }
                passes
            })
            .collect();
        let mut fresh_buttons: Vec<MouseButtonInput> =
            buttons_read.read(&buttons).copied().collect();
        if dropped.is_empty() {
            return;
        }
        for input in dropped {
            if let Some(index) = fresh_buttons.iter().position(|&b| b == input) {
                fresh_buttons.remove(index);
            }
        }
        // Readers have seen the messages from before this frame already.
        window_events.clear();
        window_events.write_batch(kept);
        *events_read = window_events.get_cursor_current();
        buttons.clear();
        buttons.write_batch(fresh_buttons);
        *buttons_read = buttons.get_cursor_current();
    }
}

#[cfg(test)]
mod tests {
    use bevy::window::WindowFocused;

    use super::*;

    const MS: Duration = Duration::from_millis(1);

    fn focus(focused: bool) -> WindowEvent {
        WindowEvent::WindowFocused(WindowFocused {
            window: Entity::PLACEHOLDER,
            focused,
        })
    }

    fn button(button: MouseButton, state: ButtonState) -> WindowEvent {
        WindowEvent::MouseButtonInput(MouseButtonInput {
            button,
            state,
            window: Entity::PLACEHOLDER,
        })
    }

    fn press() -> WindowEvent {
        button(MouseButton::Left, ButtonState::Pressed)
    }

    fn release() -> WindowEvent {
        button(MouseButton::Left, ButtonState::Released)
    }

    /// Which of `events`, each at a time in milliseconds, get through.
    fn passing(events: &[(u64, WindowEvent)]) -> Vec<bool> {
        let mut refocus = RefocusClick::default();
        events
            .iter()
            .map(|(ms, event)| refocus.passes(event, MS * *ms as u32))
            .collect()
    }

    #[test]
    fn the_click_right_after_focus_is_dropped() {
        let events = [
            (0, focus(false)),
            (5000, focus(true)),
            (5000, press()),
            (5080, release()),
            (5400, press()),
            (5480, release()),
        ];
        assert_eq!(passing(&events), [true, true, false, false, true, true]);
    }

    #[test]
    fn a_press_before_the_focus_is_dropped_alone() {
        let events = [
            (0, focus(false)),
            (5000, press()),
            (5000, focus(true)),
            (5080, release()),
            (5100, press()),
            (5180, release()),
        ];
        assert_eq!(passing(&events), [true, false, true, false, true, true]);
    }

    #[test]
    fn a_late_click_and_a_held_button_get_through() {
        let events = [
            (0, press()),
            (100, focus(false)),
            (5000, focus(true)),
            (5050, release()),
            (5400, press()),
            (5480, release()),
        ];
        assert_eq!(passing(&events), [true, true, true, true, true, true]);
    }

    #[test]
    fn other_buttons_keep_their_releases() {
        let right = |state| button(MouseButton::Right, state);
        let events = [
            (0, right(ButtonState::Pressed)),
            (10, focus(false)),
            (20, focus(true)),
            (30, press()),
            (40, right(ButtonState::Released)),
            (50, release()),
        ];
        assert_eq!(passing(&events), [true, true, true, false, true, false]);
    }

    #[test]
    fn dropped_clicks_vanish_from_both_messages() {
        let mut world = World::new();
        world.init_resource::<Messages<WindowEvent>>();
        world.init_resource::<Messages<MouseButtonInput>>();
        world.init_resource::<Time<Real>>();
        world.init_resource::<RefocusClick>();
        let mut reader = world.resource::<Messages<MouseButtonInput>>().get_cursor();
        for event in [focus(false), focus(true), press(), release()] {
            if let WindowEvent::MouseButtonInput(input) = event {
                world.write_message(input);
            }
            world.write_message(event);
        }
        world.run_system_cached(RefocusClick::drop_click).unwrap();
        let events = world.resource::<Messages<WindowEvent>>();
        assert_eq!(events.len(), 2);
        let buttons = world.resource::<Messages<MouseButtonInput>>();
        assert_eq!(reader.read(buttons).count(), 0);

        if let WindowEvent::MouseButtonInput(input) = press() {
            world.write_message(input);
        }
        world.write_message(press());
        world.run_system_cached(RefocusClick::drop_click).unwrap();
        let buttons = world.resource::<Messages<MouseButtonInput>>();
        assert_eq!(reader.read(buttons).count(), 1);
    }
}
