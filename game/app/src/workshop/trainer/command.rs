use bevy::prelude::*;
use bevy::remote::BrpResult;
use ld60_game_core::debug_port::{DebugMethod, DebugMethods, ParamKind};
use ld60_game_parts::dip::DipSwitch;
use ld60_game_parts::toggle::Toggle;
use serde_json::{Value, json};

use super::{Control, Trainer};

/// Registers `ld60.panel`.
pub(super) fn register(app: &mut App) {
    app.add_debug_method(
        DebugMethod::new(
            "ld60.panel",
            "Sets the switches of the front panel. Returns its movement.",
        )
        .param(
            "words",
            ParamKind::Numbers,
            "a word for each row of switches, from word 0",
        )
        .param("run", ParamKind::Flag, "the RUN switch"),
        panel,
    );
}

fn panel(
    In(params): In<Option<Value>>,
    trainers: Query<&Trainer>,
    mut bits: Query<(&Control, &mut DipSwitch)>,
    mut toggles: Query<(&Control, &mut Toggle)>,
) -> BrpResult {
    let params = params.unwrap_or_default();
    let words: Option<Vec<u64>> = params
        .get("words")
        .and_then(Value::as_array)
        .map(|words| words.iter().filter_map(Value::as_u64).collect());
    let run = params.get("run").and_then(Value::as_bool);
    for (control, mut switch) in &mut bits {
        if let Control::Bit { word, bit } = *control
            && let Some(value) = words.as_ref().and_then(|words| words.get(word))
        {
            switch.on = value >> bit & 1 != 0;
        }
    }
    for (control, mut toggle) in &mut toggles {
        if let (Control::Run, Some(up)) = (control, run) {
            toggle.up = up;
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
