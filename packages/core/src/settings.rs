mod frame_rate;
mod quality;

use bevy::prelude::*;
use ld60_locale::Message;
use ld60_store::{Slot, Store};
use serde::{Deserialize, Serialize};

use crate::audio::Mixer;

use self::frame_rate::FrameEnd;
pub use self::frame_rate::FrameRate;
use self::quality::Quality;

const VOLUME_STEPS: u8 = 10;
/// Mouse sensitivity goes from one step to this many, in quarters.
const SENSITIVITY_STEPS: u8 = 12;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Store>();
        let settings = app
            .world()
            .resource::<Store>()
            .load::<Settings>(Slot::Settings)
            .map(Settings::checked)
            .unwrap_or_default();
        app.insert_resource(settings)
            .init_resource::<FrameEnd>()
            .add_systems(Update, Settings::apply.run_if(resource_changed::<Settings>))
            .add_systems(Last, FrameEnd::wait)
            .add_systems(Last, Settings::save.run_if(resource_changed::<Settings>))
            .add_observer(Adjust::apply);
    }
}

/// What the player set in the settings menu.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// In tenths of full volume.
    volume: u8,
    /// In quarters of the normal speed.
    sensitivity: u8,
    /// Whether the mouse buttons and keys to use are shown.
    hints: bool,
    quality: Quality,
    #[serde(default)]
    frame_rate: FrameRate,
}

/// One of the [`Settings`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Volume,
    Sensitivity,
    Hints,
    Quality,
    FrameRate,
}

/// Steps a [`Setting`] up or down.
#[derive(Event, Clone, Copy, Debug)]
pub struct Adjust {
    pub setting: Setting,
    pub step: i32,
    /// Goes round to the other end instead of stopping there.
    pub wrap: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 8,
            sensitivity: 4,
            hints: true,
            quality: Quality::default(),
            frame_rate: FrameRate::default(),
        }
    }
}

impl Settings {
    /// From 0 to 1.
    pub fn volume(&self) -> f32 {
        f32::from(self.volume) / f32::from(VOLUME_STEPS)
    }

    /// 1 is the normal speed of mouse look.
    pub fn sensitivity(&self) -> f32 {
        f32::from(self.sensitivity) / 4.0
    }

    pub const fn hints(&self) -> bool {
        self.hints
    }

    pub const fn quality(&self) -> Quality {
        self.quality
    }

    fn apply(settings: Res<Settings>, mut mixer: ResMut<Mixer>) {
        mixer.volume = settings.volume();
    }

    /// The settings with each value moved into its range, in case the save
    /// was edited by hand.
    fn checked(self) -> Self {
        Self {
            volume: self.volume.min(VOLUME_STEPS),
            sensitivity: self.sensitivity.clamp(1, SENSITIVITY_STEPS),
            ..self
        }
    }

    fn save(settings: Res<Settings>, store: Res<Store>) {
        store.save(Slot::Settings, &*settings);
    }

    /// The default settings with the key hints off, for tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn toggled_hints() -> Self {
        Self {
            hints: false,
            ..Self::default()
        }
    }

    pub fn value(&self, setting: Setting) -> Message {
        match setting {
            Setting::Volume => Message::new("settings-percent").with(
                "value",
                u32::from(self.volume) * 100 / u32::from(VOLUME_STEPS),
            ),
            Setting::Sensitivity => {
                Message::new("settings-percent").with("value", u32::from(self.sensitivity) * 25)
            }
            Setting::Hints if self.hints => Message::new("settings-on"),
            Setting::Hints => Message::new("settings-off"),
            Setting::Quality => Message::new(self.quality.label()),
            Setting::FrameRate => Message::new(self.frame_rate.label()),
        }
    }
}

impl Setting {
    pub const ALL: [Self; 5] = [
        Self::Volume,
        Self::Sensitivity,
        Self::Hints,
        Self::Quality,
        Self::FrameRate,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Volume => "settings-volume",
            Self::Sensitivity => "settings-sensitivity",
            Self::Hints => "settings-hints",
            Self::Quality => "settings-quality",
            Self::FrameRate => "settings-frame-rate",
        }
    }
}

impl Adjust {
    fn apply(adjust: On<Adjust>, mut settings: ResMut<Settings>) {
        let step = |value: u8, low: u8, high: u8| {
            let next = i32::from(value) + adjust.step;
            let next = if adjust.wrap && next > i32::from(high) {
                i32::from(low)
            } else if adjust.wrap && next < i32::from(low) {
                i32::from(high)
            } else {
                next.clamp(i32::from(low), i32::from(high))
            };
            u8::try_from(next).unwrap_or(low)
        };
        match adjust.setting {
            Setting::Volume => settings.volume = step(settings.volume, 0, VOLUME_STEPS),
            Setting::Sensitivity => {
                settings.sensitivity = step(settings.sensitivity, 1, SENSITIVITY_STEPS);
            }
            Setting::Hints => settings.hints = !settings.hints,
            Setting::Quality => {
                let levels = Quality::ALL.len() as u8 - 1;
                let index = Quality::ALL
                    .iter()
                    .position(|quality| *quality == settings.quality)
                    .unwrap_or_default();
                let index = step(u8::try_from(index).unwrap_or_default(), 0, levels);
                settings.quality = Quality::ALL[usize::from(index)];
            }
            Setting::FrameRate => {
                let levels = FrameRate::ALL.len() as u8 - 1;
                let index = FrameRate::ALL
                    .iter()
                    .position(|rate| *rate == settings.frame_rate)
                    .unwrap_or_default();
                let index = step(u8::try_from(index).unwrap_or_default(), 0, levels);
                settings.frame_rate = FrameRate::ALL[usize::from(index)];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_from_before_the_frame_rate_load_at_60() {
        let old = "(volume:8,sensitivity:4,hints:true,quality:Low)";
        let settings: Settings = ron::from_str(old).unwrap();
        assert_eq!(settings.frame_rate, FrameRate::Sixty);
        assert_eq!(settings.quality, Quality::Low);
    }

    #[test]
    fn a_change_saves_the_settings() {
        let mut world = World::new();
        world.insert_resource(Store::in_temp("settings"));
        world.init_resource::<Settings>();
        world.add_observer(Adjust::apply);
        world.trigger(Adjust {
            setting: Setting::Quality,
            step: 1,
            wrap: true,
        });
        world.run_system_cached(Settings::save).unwrap();
        let saved = world.resource::<Store>().load::<Settings>(Slot::Settings);
        assert_eq!(saved, Some(*world.resource::<Settings>()));
        assert_ne!(saved, Some(Settings::default()));
    }

    #[test]
    fn out_of_range_settings_are_moved_back() {
        let settings = Settings {
            volume: 200,
            sensitivity: 0,
            ..Settings::default()
        }
        .checked();
        assert_eq!((settings.volume, settings.sensitivity), (VOLUME_STEPS, 1));
        assert_eq!(Settings::default().checked(), Settings::default());
    }
}
