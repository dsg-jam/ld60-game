mod input;
mod profile;

#[cfg(not(target_arch = "wasm32"))]
use bevy::remote::http::{DEFAULT_ADDR, RemoteHttpPlugin};
use bevy::{
    ecs::system::IntoSystem,
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods, RemotePlugin, error_codes},
    render::view::screenshot::{Screenshot, save_to_disk},
    window::PrimaryWindow,
};
use serde_json::{Value, json};

use crate::state::KeepMouse;

const PORT_VARIABLE: &str = "LD60_DEBUG_PORT";
const KEEP_MOUSE_VARIABLE: &str = "LD60_KEEP_MOUSE";

/// Serves the Bevy Remote Protocol when `LD60_DEBUG_PORT` is set.
///
/// With `LD60_KEEP_MOUSE` also set, the game leaves the system mouse to the
/// user, see [`KeepMouse`]. Its window opens in the background, and
/// `ld60.input` only moves the pointer of the game.
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
        .add_debug_method("ld60.where", place)
        .add_debug_method("ld60.time", speed)
        .add_debug_method("ld60.input", input::input)
        .add_debug_method("ld60.diagnostics", profile::diagnostics)
        .add_debug_method("ld60.vsync", profile::vsync);
        info!("Debug port listening on {DEFAULT_ADDR}:{port}");
        if std::env::var_os(KEEP_MOUSE_VARIABLE).is_some() {
            let world = app.world_mut();
            let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
            for mut window in windows.iter_mut(world) {
                window.focused = false;
            }
            app.insert_resource(KeepMouse);
        }
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

/// `ld60.where { "name": "stamp" }` returns each entity whose name
/// contains `name`, with its name and where it is in the world. Without a
/// name, every named entity.
fn place(In(params): In<Option<Value>>, entities: Query<(&Name, &GlobalTransform)>) -> BrpResult {
    let filter = params
        .as_ref()
        .and_then(|params| params.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let places: Vec<_> = entities
        .iter()
        .filter(|(name, _)| name.as_str().contains(filter))
        .map(|(name, transform)| {
            let at = transform.translation();
            json!({ "name": name.as_str(), "at": [at.x, at.y, at.z] })
        })
        .collect();
    Ok(places.into())
}

/// `ld60.time { "speed": 0.1 }` runs game time at `speed` times real time.
/// 0 freezes it. Returns the speed.
fn speed(In(params): In<Option<Value>>, mut time: ResMut<Time<Virtual>>) -> BrpResult {
    let speed = params
        .as_ref()
        .and_then(|params| params.get("speed"))
        .and_then(Value::as_f64)
        .filter(|speed| speed.is_finite() && *speed >= 0.0);
    if let Some(speed) = speed {
        time.set_relative_speed(speed as f32);
    }
    Ok(json!(time.relative_speed()))
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
