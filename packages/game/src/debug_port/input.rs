use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput, NativeKey, NativeKeyCode},
        mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
    },
    prelude::*,
    remote::{BrpError, BrpResult, error_codes},
    window::{CursorMoved, PrimaryWindow, WindowEvent},
};
use serde_json::Value;

/// `ld60.input { "pointer": [x, y], "click": "left", "scroll": -1,
/// "key": "ArrowRight", "text": "LD M1" }` feeds input into the game as if it
/// came from the window.
///
/// All fields are optional and apply in that order. `pointer` is in logical
/// pixels and also moves the system cursor. `click` is `left` or `right`.
/// `scroll` turns the wheel by that many lines, negative is down. A list of
/// numbers scrolls by that many pixels each, like a trackpad.
/// `key` is a `KeyCode` name. `text` types each character. Send `pointer`
/// in an earlier call than `click`, so the click sees what is under it.
/// Device and computer keys see a key once per frame, so send repeated
/// presses of the same key in separate calls.
pub(super) fn input(
    In(params): In<Option<Value>>,
    window: Single<(Entity, &mut Window), With<PrimaryWindow>>,
    mut window_events: MessageWriter<WindowEvent>,
    mut cursor_moves: MessageWriter<CursorMoved>,
    mut buttons: MessageWriter<MouseButtonInput>,
    mut wheels: MessageWriter<MouseWheel>,
    mut keys: MessageWriter<KeyboardInput>,
) -> BrpResult {
    let (entity, mut window) = window.into_inner();
    let params = params.unwrap_or_default();

    if let Some(pointer) = params.get("pointer") {
        let position = pointer
            .as_array()
            .and_then(|xy| {
                Some(Vec2::new(
                    xy.first()?.as_f64()? as f32,
                    xy.get(1)?.as_f64()? as f32,
                ))
            })
            .ok_or_else(|| invalid("`pointer` must be [x, y]"))?;
        window.set_cursor_position(Some(position));
        let moved = CursorMoved {
            window: entity,
            position,
            delta: None,
        };
        window_events.write(WindowEvent::CursorMoved(moved.clone()));
        cursor_moves.write(moved);
    }

    if let Some(click) = params.get("click") {
        let button = match click.as_str() {
            Some("left") => MouseButton::Left,
            Some("right") => MouseButton::Right,
            _ => return Err(invalid("`click` must be \"left\" or \"right\"")),
        };
        for state in [ButtonState::Pressed, ButtonState::Released] {
            let input = MouseButtonInput {
                button,
                state,
                window: entity,
            };
            window_events.write(WindowEvent::MouseButtonInput(input));
            buttons.write(input);
        }
    }

    if let Some(scroll) = params.get("scroll") {
        let steps = match scroll {
            Value::Array(pixels) => pixels
                .iter()
                .map(|y| Some((MouseScrollUnit::Pixel, y.as_f64()? as f32)))
                .collect::<Option<Vec<_>>>(),
            lines => lines
                .as_f64()
                .map(|y| vec![(MouseScrollUnit::Line, y as f32)]),
        }
        .ok_or_else(|| invalid("`scroll` must be a number or a list of numbers"))?;
        for (unit, y) in steps {
            let wheel = MouseWheel {
                unit,
                x: 0.0,
                y,
                window: entity,
                phase: TouchPhase::Moved,
            };
            window_events.write(WindowEvent::MouseWheel(wheel));
            wheels.write(wheel);
        }
    }

    let mut typed = Vec::new();
    if let Some(key) = params.get("key") {
        let key_code = serde_json::from_value::<KeyCode>(key.clone())
            .map_err(|_| invalid("`key` must be a KeyCode name like \"ArrowRight\""))?;
        let logical_key = serde_json::from_value::<Key>(key.clone())
            .unwrap_or(Key::Unidentified(NativeKey::Unidentified));
        typed.push((key_code, logical_key, None));
    }
    if let Some(text) = params.get("text") {
        let text = text
            .as_str()
            .ok_or_else(|| invalid("`text` must be a string"))?;
        typed.extend(text.chars().map(character));
    }
    for (key_code, logical_key, text) in typed {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            let input = KeyboardInput {
                key_code,
                logical_key: logical_key.clone(),
                state,
                text: text
                    .as_deref()
                    .filter(|_| state.is_pressed())
                    .map(Into::into),
                repeat: false,
                window: entity,
            };
            window_events.write(WindowEvent::KeyboardInput(input.clone()));
            keys.write(input);
        }
    }
    Ok(Value::Null)
}

/// The physical key, logical key and text for typing `c`.
fn character(c: char) -> (KeyCode, Key, Option<String>) {
    match c {
        '\n' => (KeyCode::Enter, Key::Enter, None),
        ' ' => (KeyCode::Space, Key::Space, Some(" ".to_owned())),
        _ => {
            let key_code = serde_json::from_value::<KeyCode>(Value::String(match c {
                'a'..='z' | 'A'..='Z' => format!("Key{}", c.to_ascii_uppercase()),
                '0'..='9' => format!("Digit{c}"),
                _ => String::new(),
            }))
            .unwrap_or(KeyCode::Unidentified(NativeKeyCode::Unidentified));
            (
                key_code,
                Key::Character(c.to_string().into()),
                Some(c.to_string()),
            )
        }
    }
}

fn invalid(message: &str) -> BrpError {
    BrpError {
        code: error_codes::INVALID_PARAMS,
        message: message.into(),
        data: None,
    }
}
