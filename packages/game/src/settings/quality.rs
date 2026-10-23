use bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel;
use serde::{Deserialize, Serialize};

/// How much the graphics card has to do for each frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Quality {
    Low,
    Medium,
    High,
}

impl Default for Quality {
    /// Browsers get less out of the graphics card, so they start lower.
    fn default() -> Self {
        if cfg!(target_arch = "wasm32") {
            Self::Low
        } else {
            Self::Medium
        }
    }
}

impl Quality {
    pub(super) const ALL: [Self; 3] = [Self::Low, Self::Medium, Self::High];

    /// Samples along each view ray through the street haze, or no haze.
    pub(crate) const fn haze_steps(self) -> Option<u32> {
        match self {
            Self::Low => None,
            Self::Medium => Some(32),
            Self::High => Some(64),
        }
    }

    /// Whether the puddles reflect the street.
    pub(crate) const fn reflections(self) -> bool {
        !matches!(self, Self::Low)
    }

    pub(crate) const fn ambient_occlusion(self) -> Option<ScreenSpaceAmbientOcclusionQualityLevel> {
        match self {
            Self::Low => None,
            Self::Medium => Some(ScreenSpaceAmbientOcclusionQualityLevel::Low),
            Self::High => Some(ScreenSpaceAmbientOcclusionQualityLevel::High),
        }
    }

    /// Share of the rain streaks that fall.
    pub(crate) const fn rain(self) -> f32 {
        match self {
            Self::Low => 0.5,
            Self::Medium | Self::High => 1.0,
        }
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Low => "settings-quality-low",
            Self::Medium => "settings-quality-medium",
            Self::High => "settings-quality-high",
        }
    }
}
