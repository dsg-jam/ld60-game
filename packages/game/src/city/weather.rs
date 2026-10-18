use bevy::prelude::*;

#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;

/// Wind at street level along X and Z, in meters per second. The rain
/// drifts with it, and the clouds with the stronger wind up high.
pub(super) const WIND: Vec2 = Vec2::new(1.2, 0.6);
/// Rain strength at the start.
const RAIN: f32 = 0.6;
/// Seconds for the ground to get soaked, and to dry again.
const SOAK_TIME: f32 = 20.0;
const DRY_TIME: f32 = 120.0;
const TOGGLE_KEY: KeyCode = KeyCode::F9;
/// Wetness changes in this many steps from dry to soaked, so everything that
/// follows it only updates now and then.
const WETNESS_STEPS: f32 = 50.0;

pub(super) struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Weather {
            rain: RAIN,
            wetness: RAIN,
            soaked: RAIN,
        })
        .add_systems(Update, (Weather::toggle_rain, Weather::settle).chain());
        #[cfg(feature = "debug_port")]
        app.add_debug_method("ld60.weather", Weather::debug_set);
    }
}

/// How hard it rains and how wet the ground is, both from 0 to 1.
///
/// The ground follows the rain slowly: it soaks in [`SOAK_TIME`] seconds and
/// dries in [`DRY_TIME`] seconds.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(super) struct Weather {
    pub(super) rain: f32,
    pub(super) wetness: f32,
    /// The exact wetness, between the steps it is shown in.
    soaked: f32,
}

impl Weather {
    fn toggle_rain(keys: Res<ButtonInput<KeyCode>>, mut weather: ResMut<Weather>) {
        if keys.just_pressed(TOGGLE_KEY) {
            weather.rain = if weather.rain > 0.0 { 0.0 } else { RAIN };
        }
    }

    fn settle(time: Res<Time>, mut weather: ResMut<Weather>) {
        let exact = weather.bypass_change_detection();
        let time_to_settle = if exact.rain > exact.soaked {
            SOAK_TIME
        } else {
            DRY_TIME
        };
        let step = time.delta_secs() / time_to_settle;
        exact.soaked += (exact.rain - exact.soaked).clamp(-step, step);
        let wetness = (exact.soaked * WETNESS_STEPS).round() / WETNESS_STEPS;
        if wetness != weather.wetness {
            weather.wetness = wetness;
        }
    }

    /// `ld60.weather {rain, wetness}` sets either value and returns both.
    #[cfg(feature = "debug_port")]
    fn debug_set(
        In(params): In<Option<serde_json::Value>>,
        mut weather: ResMut<Weather>,
    ) -> bevy::remote::BrpResult {
        let get = |key: &str| {
            params
                .as_ref()
                .and_then(|params| params.get(key))
                .and_then(serde_json::Value::as_f64)
                .map(|value| (value as f32).clamp(0.0, 1.0))
        };
        if let Some(rain) = get("rain") {
            weather.rain = rain;
        }
        if let Some(wetness) = get("wetness") {
            weather.wetness = wetness;
            weather.soaked = wetness;
        }
        Ok(serde_json::json!({ "rain": weather.rain, "wetness": weather.wetness }))
    }
}
