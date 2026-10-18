use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};

use crate::audio::{Play, Sound};
use crate::player::focus::{Clickable, Focus};

pub(crate) struct SwitchPlugin;

impl Plugin for SwitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(LightSwitch::toggle_on_press);
    }
}

/// Switches a light and its glowing bulb when clicked in the desk view.
#[derive(Component, Debug)]
#[require(Clickable)]
pub(crate) struct LightSwitch {
    on: bool,
    light: Entity,
    intensity: f32,
    bulb: Handle<StandardMaterial>,
    glow: LinearRgba,
}

impl LightSwitch {
    /// A switch that starts out on.
    pub(crate) fn new(
        light: Entity,
        intensity: f32,
        bulb: Handle<StandardMaterial>,
        glow: LinearRgba,
    ) -> Self {
        Self {
            on: true,
            light,
            intensity,
            bulb,
            glow,
        }
    }

    fn toggle_on_press(
        mut press: On<PointerPress>,
        state: Res<State<Focus>>,
        mut switches: Query<&mut LightSwitch>,
        mut lights: Query<&mut PointLight>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || *state.get() != Focus::Desk {
            return;
        }
        let Ok(mut switch) = switches.get_mut(press.entity) else {
            return;
        };
        press.propagate(false);
        switch.on = !switch.on;
        if let Ok(mut light) = lights.get_mut(switch.light) {
            light.intensity = if switch.on { switch.intensity } else { 0.0 };
        }
        if let Some(mut bulb) = materials.get_mut(&switch.bulb) {
            bulb.emissive = if switch.on {
                switch.glow
            } else {
                LinearRgba::BLACK
            };
        }
        commands.trigger(Play(Sound::Click));
    }
}
