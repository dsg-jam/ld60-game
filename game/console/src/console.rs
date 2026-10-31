use std::collections::VecDeque;

use bevy::ecs::message::MessageCursor;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::remote::{RemoteMethodSystemId, RemoteMethods};
use ld60_game_core::debug_port::DebugMethodList;

use super::complete::Completion;
use super::help::Help;
use super::line::Call;

/// The key that opens and closes the console, left of 1.
const TOGGLE: KeyCode = KeyCode::Backquote;
/// Older output is dropped.
const KEPT_LINES: usize = 400;
/// Results up to this long are printed on one line.
const ONE_LINE: usize = 120;
/// Page Up and Page Down scroll by this many lines.
const PAGE: usize = 10;

/// The console: the line being typed, the output and the history.
///
/// While it is open it takes all keyboard input, so the game sees none.
#[derive(Resource, Debug, Default)]
pub(crate) struct Console {
    pub(crate) open: bool,
    pub(crate) line: String,
    /// A byte offset into `line`.
    pub(crate) caret: usize,
    output: VecDeque<String>,
    /// Lines hidden at the bottom of the output, scrolled up.
    pub(crate) scroll: usize,
    history: Vec<String>,
    /// The history entry shown with Up and Down.
    browsing: Option<usize>,
    pending: Vec<String>,
}

impl Console {
    /// Reads the keyboard before anything else does. While open, or on the
    /// frame it opens or closes, the keys are taken away from the game, and
    /// keys held when it opens are let go.
    pub(crate) fn read_keys(
        mut messages: ResMut<Messages<KeyboardInput>>,
        mut cursor: Local<MessageCursor<KeyboardInput>>,
        mut codes: ResMut<ButtonInput<KeyCode>>,
        mut keys: ResMut<ButtonInput<Key>>,
        methods: Res<DebugMethodList>,
        mut console: ResMut<Console>,
    ) {
        let inputs: Vec<KeyboardInput> = cursor.read(&messages).cloned().collect();
        let was_open = console.open;
        for input in inputs {
            if input.state != ButtonState::Pressed {
                continue;
            }
            if input.key_code == TOGGLE {
                if !input.repeat {
                    console.open = !console.open;
                }
                continue;
            }
            if console.open {
                console.press(&input, &methods);
            }
        }
        if was_open || console.open {
            messages.clear();
            codes.reset_all();
            keys.reset_all();
        }
    }

    /// Runs the lines entered since the last frame.
    pub(crate) fn run(world: &mut World) {
        let pending = std::mem::take(&mut world.resource_mut::<Console>().pending);
        for line in pending {
            let output = Self::execute(world, &line);
            let mut console = world.resource_mut::<Console>();
            console.print(&format!("> {line}"));
            if let Some(output) = output {
                console.print(&output);
            }
        }
    }

    /// The output, oldest first, without the lines scrolled away.
    pub(crate) fn output(&self) -> impl Iterator<Item = &str> {
        let shown = self.output.len().saturating_sub(self.scroll);
        self.output.iter().take(shown).map(String::as_str)
    }

    fn press(&mut self, input: &KeyboardInput, methods: &DebugMethodList) {
        match &input.logical_key {
            Key::Enter => self.submit(),
            Key::Escape => self.open = false,
            Key::Tab => {
                self.caret = self.line.len();
                let completion = Completion::of(&self.line, methods);
                if !completion.options.is_empty() {
                    self.print(&completion.options.join("  "));
                }
                self.line = completion.line;
                self.caret = self.line.len();
            }
            Key::Backspace => {
                if let Some(c) = self.line[..self.caret].chars().next_back() {
                    self.caret -= c.len_utf8();
                    self.line.remove(self.caret);
                }
            }
            Key::Delete => {
                if self.caret < self.line.len() {
                    self.line.remove(self.caret);
                }
            }
            Key::ArrowLeft => {
                if let Some(c) = self.line[..self.caret].chars().next_back() {
                    self.caret -= c.len_utf8();
                }
            }
            Key::ArrowRight => {
                if let Some(c) = self.line[self.caret..].chars().next() {
                    self.caret += c.len_utf8();
                }
            }
            Key::Home => self.caret = 0,
            Key::End => self.caret = self.line.len(),
            Key::ArrowUp => self.browse(-1),
            Key::ArrowDown => self.browse(1),
            Key::PageUp => {
                self.scroll = (self.scroll + PAGE).min(self.output.len().saturating_sub(1));
            }
            Key::PageDown => self.scroll = self.scroll.saturating_sub(PAGE),
            _ => {
                let Some(text) = &input.text else {
                    return;
                };
                for c in text.chars().filter(|c| !c.is_control()) {
                    self.line.insert(self.caret, c);
                    self.caret += c.len_utf8();
                }
            }
        }
    }

    fn submit(&mut self) {
        let line = std::mem::take(&mut self.line);
        self.caret = 0;
        self.browsing = None;
        self.scroll = 0;
        if !line.trim().is_empty() && self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        self.pending.push(line);
    }

    /// Steps `by` entries through the history. Past the newest entry the
    /// line is empty.
    fn browse(&mut self, by: isize) {
        let newest = self.history.len();
        let at = self
            .browsing
            .unwrap_or(newest)
            .saturating_add_signed(by)
            .min(newest);
        self.browsing = (at < newest).then_some(at);
        self.line = self.history.get(at).cloned().unwrap_or_default();
        self.caret = self.line.len();
    }

    fn print(&mut self, text: &str) {
        self.output.extend(text.lines().map(str::to_owned));
        let extra = self.output.len().saturating_sub(KEPT_LINES);
        self.output.drain(..extra);
    }

    fn execute(world: &mut World, line: &str) -> Option<String> {
        let methods = world.resource::<DebugMethodList>();
        let mut words = line.split_whitespace();
        match (words.next(), words.next()) {
            (Some("help"), None) => return Some(Help::All(methods).to_string()),
            (Some("help"), Some(name)) => {
                return Some(match methods.get(&Call::full_name(name)) {
                    Some(method) => Help::Method(method).to_string(),
                    None => format!("no command {name}"),
                });
            }
            (None, _) => return None,
            (Some("clear"), None) => {
                world.resource_mut::<Console>().output.clear();
                return None;
            }
            _ => {}
        }
        let method = Call::full_name(line.split_whitespace().next().unwrap_or_default());
        let id = match world.resource::<RemoteMethods>().get(&method) {
            Some(RemoteMethodSystemId::Instant(id)) => *id,
            Some(RemoteMethodSystemId::Watching(_)) => {
                return Some(format!("{method} watches, the console only calls"));
            }
            None => return Some(format!("no command {method}, try help")),
        };
        let call = match Call::parse(line, world.resource::<DebugMethodList>()) {
            Ok(call) => call,
            Err(error) => return Some(error.to_string()),
        };
        Some(match world.run_system_with(id, call.params) {
            Ok(Ok(serde_json::Value::Null)) => "ok".to_owned(),
            Ok(Ok(value)) => {
                let line = value.to_string();
                if line.len() <= ONE_LINE {
                    line
                } else {
                    serde_json::to_string_pretty(&value).unwrap_or(line)
                }
            }
            Ok(Err(error)) => format!("error: {}", error.message),
            Err(error) => format!("error: {error}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_and_down_step_through_the_history() {
        let mut console = Console::default();
        for line in ["a", "b", "b", "c"] {
            console.line = line.to_owned();
            console.submit();
        }
        assert_eq!(console.history, ["a", "b", "c"]);
        console.browse(-1);
        console.browse(-1);
        assert_eq!(console.line, "b");
        console.browse(-1);
        console.browse(-1);
        assert_eq!(console.line, "a");
        console.browse(1);
        console.browse(1);
        console.browse(1);
        assert_eq!(console.line, "");
    }

    #[test]
    fn old_output_is_dropped() {
        let mut console = Console::default();
        for line in 0..KEPT_LINES + 5 {
            console.print(&line.to_string());
        }
        assert_eq!(console.output().next(), Some("5"));
        console.scroll = 2;
        assert_eq!(
            console.output().last(),
            Some(&*(KEPT_LINES + 2).to_string())
        );
    }
}
