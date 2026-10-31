use bevy::{
    diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    prelude::*,
    remote::BrpResult,
    render::diagnostic::RenderDiagnosticsPlugin,
    window::{PresentMode, PrimaryWindow},
};
use serde_json::{Map, Value, json};

/// Measures frame and render pass times for the debug port.
#[cfg_attr(
    target_arch = "wasm32",
    expect(dead_code, reason = "the port is not served on the web")
)]
pub(super) struct ProfilePlugin;

impl Plugin for ProfilePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            RenderDiagnosticsPlugin,
        ));
    }
}

/// `ld60.diagnostics` returns the average and latest value of every
/// diagnostic, by path.
pub(super) fn diagnostics(In(_): In<Option<Value>>, store: Res<DiagnosticsStore>) -> BrpResult {
    let values: Map<String, Value> = store
        .iter()
        .map(|diagnostic| {
            let value = json!({
                "average": diagnostic.average(),
                "value": diagnostic.value(),
                "samples": diagnostic.history_len(),
            });
            (diagnostic.path().as_str().to_owned(), value)
        })
        .collect();
    Ok(values.into())
}

/// `ld60.vsync { "on": false }` lets the frame rate run free.
pub(super) fn vsync(
    In(params): In<Option<Value>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) -> BrpResult {
    let on = params
        .as_ref()
        .and_then(|params| params.get("on"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    window.present_mode = if on {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
    Ok(json!({ "on": on }))
}
