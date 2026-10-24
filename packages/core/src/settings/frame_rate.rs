use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde::{Deserialize, Serialize};

use super::Settings;

/// The shortest frame while the window is in the background.
const BACKGROUND_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 15);

/// The most frames the game draws each second.
///
/// Fast displays would otherwise make the game draw up to 120 frames a
/// second and keep laptops hot for little gain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameRate {
    Thirty,
    #[default]
    Sixty,
    /// As many as the display shows.
    Unlimited,
}

/// When the last frame ended.
#[derive(Resource, Debug)]
pub(super) struct FrameEnd(Instant);

impl FrameRate {
    pub(super) const ALL: [Self; 3] = [Self::Thirty, Self::Sixty, Self::Unlimited];

    /// The shortest time a frame may take.
    pub const fn interval(self) -> Option<Duration> {
        match self {
            Self::Thirty => Some(Duration::from_nanos(1_000_000_000 / 30)),
            Self::Sixty => Some(Duration::from_nanos(1_000_000_000 / 60)),
            Self::Unlimited => None,
        }
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Thirty => "settings-frame-rate-30",
            Self::Sixty => "settings-frame-rate-60",
            Self::Unlimited => "settings-frame-rate-unlimited",
        }
    }
}

impl FrameEnd {
    /// Waits until the frame took at least the interval of the frame rate,
    /// or longer while the window is in the background.
    ///
    /// Browsers pace frames themselves and cannot sleep, so this does
    /// nothing there.
    pub(super) fn wait(
        settings: Res<Settings>,
        windows: Query<&Window, With<PrimaryWindow>>,
        mut end: ResMut<FrameEnd>,
    ) {
        let background = windows.single().is_ok_and(|window| !window.focused);
        let interval = if background {
            Some(BACKGROUND_INTERVAL)
        } else {
            settings.frame_rate.interval()
        };
        if cfg!(not(target_arch = "wasm32"))
            && let Some(interval) = interval
        {
            let rest = interval.saturating_sub(end.0.elapsed());
            if !rest.is_zero() {
                std::thread::sleep(rest);
            }
        }
        end.0 = Instant::now();
    }
}

impl Default for FrameEnd {
    fn default() -> Self {
        Self(Instant::now())
    }
}
