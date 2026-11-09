use bevy::prelude::*;
use bevy::remote::{BrpError, BrpResult, error_codes};
use ld60_game_core::debug_port::{DebugMethod, DebugMethods, ParamKind};
use ld60_game_parts::push_button::Pushed;
use ld60_game_parts::toggle::Toggle;
use serde_json::{Value, json};

use super::{Control, Trainer};

const BUTTONS: [(&str, Control); 5] = [
    ("reset", Control::Reset),
    ("erase", Control::Erase),
    ("next", Control::Next),
    ("deposit", Control::Deposit),
    ("step", Control::Step),
];

/// Registers `ld60.panel`.
pub(super) fn register(app: &mut App) {
    app.add_debug_method(
        DebugMethod::new(
            "ld60.panel",
            "Sets the switches of the front panel and pushes its buttons. Returns its movement.",
        )
        .param(
            "switches",
            ParamKind::Numbers,
            "the word on the data switches",
        )
        .param("run", ParamKind::Flag, "the RUN switch")
        .param(
            "press",
            ParamKind::Choice(&["reset", "erase", "next", "deposit", "step"]),
            "a button",
        ),
        panel,
    );
}

fn panel(
    In(params): In<Option<Value>>,
    trainers: Query<&Trainer>,
    mut controls: Query<(Entity, &Control, Option<&mut Toggle>)>,
    mut commands: Commands,
) -> BrpResult {
    let params = params.unwrap_or_default();
    let switches = params.get("switches").and_then(Value::as_u64);
    let run = params.get("run").and_then(Value::as_bool);
    for (_, control, toggle) in &mut controls {
        let up = match control {
            Control::Data(bit) => switches.map(|word| word >> bit & 1 != 0),
            Control::Run => run,
            _ => None,
        };
        if let (Some(up), Some(mut toggle)) = (up, toggle) {
            toggle.up = up;
        }
    }
    if let Some(name) = params.get("press").and_then(Value::as_str) {
        let Some((_, button)) = BUTTONS.iter().find(|(known, _)| *known == name) else {
            return Err(BrpError {
                code: error_codes::INVALID_PARAMS,
                message: format!("no button {name}"),
                data: None,
            });
        };
        for (entity, control, _) in &controls {
            if control == button {
                commands.trigger(Pushed { entity });
            }
        }
    }
    let movement = trainers
        .iter()
        .find_map(|trainer| trainer.movement.as_ref());
    Ok(json!(movement.map(|movement| {
        json!({
            "rom": movement.rom().image().words(),
            "pc": movement.cpu().pc(),
            "hand": movement.hand(),
        })
    })))
}
