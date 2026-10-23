mod input;
mod profile;

#[cfg(not(target_arch = "wasm32"))]
use bevy::remote::http::{DEFAULT_ADDR, RemoteHttpPlugin};
use bevy::{
    ecs::system::IntoSystem,
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods, RemotePlugin, error_codes},
    render::view::screenshot::{Screenshot, save_to_disk},
    window::{CursorOptions, PrimaryWindow},
};
use serde_json::{Value, json};

use crate::player::focus::Focus;
use crate::state::AppState;

const PORT_VARIABLE: &str = "LD60_DEBUG_PORT";

/// Serves the Bevy Remote Protocol when `LD60_DEBUG_PORT` is set.
pub struct DebugPortPlugin;

impl Plugin for DebugPortPlugin {
    #[cfg(target_arch = "wasm32")]
    fn build(&self, _app: &mut App) {}

    #[cfg(not(target_arch = "wasm32"))]
    fn build(&self, app: &mut App) {
        let Ok(port) = std::env::var(PORT_VARIABLE) else {
            return;
        };
        let Ok(port) = port.parse() else {
            warn!("{PORT_VARIABLE} is not a port number: {port}");
            return;
        };
        app.add_plugins((
            RemotePlugin::default(),
            RemoteHttpPlugin::default().with_port(port),
            profile::ProfilePlugin,
        ))
        .add_debug_method("ld60.screenshot", screenshot)
        .add_debug_method("ld60.state", state)
        .add_debug_method("ld60.input", input::input)
        .add_debug_method("ld60.diagnostics", profile::diagnostics)
        .add_debug_method("ld60.vsync", profile::vsync);
        info!("Debug port listening on {DEFAULT_ADDR}:{port}");
    }
}

/// Registers `ld60.*` methods on the debug port, if it is running.
pub trait DebugMethods {
    fn add_debug_method<M>(
        &mut self,
        name: &str,
        handler: impl IntoSystem<In<Option<Value>>, BrpResult, M> + 'static,
    ) -> &mut Self;
}

impl DebugMethods for App {
    fn add_debug_method<M>(
        &mut self,
        name: &str,
        handler: impl IntoSystem<In<Option<Value>>, BrpResult, M> + 'static,
    ) -> &mut Self {
        if self.world().contains_resource::<RemoteMethods>() {
            let id = self.world_mut().register_system(handler);
            self.world_mut()
                .resource_mut::<RemoteMethods>()
                .insert(name, RemoteMethodSystemId::Instant(id));
        }
        self
    }
}

/// `ld60.state` returns the game state, the focus, whether the window has
/// focus and the cursor grab mode.
fn state(
    In(_): In<Option<Value>>,
    app_state: Res<State<AppState>>,
    focus: Res<State<Focus>>,
    window: Single<(&Window, &CursorOptions), With<PrimaryWindow>>,
) -> BrpResult {
    let (window, cursor) = *window;
    Ok(json!({
        "app": format!("{:?}", app_state.get()),
        "focus": format!("{:?}", focus.get()),
        "window_focused": window.focused,
        "cursor": format!("{:?}", cursor.grab_mode),
    }))
}

/// `ld60.screenshot { "path": "..." }` saves the next frame to `path`.
fn screenshot(In(params): In<Option<Value>>, mut commands: Commands) -> BrpResult {
    let path = params
        .as_ref()
        .and_then(|params| params.get("path"))
        .and_then(Value::as_str)
        .ok_or_else(|| BrpError {
            code: error_codes::INVALID_PARAMS,
            message: "expected a string `path`".into(),
            data: None,
        })?
        .to_owned();
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()));
    Ok(json!({ "path": path }))
}
