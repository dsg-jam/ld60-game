pub mod bulb;
pub mod dot_matrix;
pub mod gauge;
pub mod keypad;
pub mod knob;
pub mod latch_button;
pub mod lcd;
pub mod liquid;
pub mod paper;
pub mod screen;
pub mod script;
pub mod steam;
pub mod sticker;

use bevy::prelude::*;

/// Reusable pieces for in-world objects.
pub struct PartsPlugin;

impl Plugin for PartsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            bulb::BulbPlugin,
            dot_matrix::DotMatrixPlugin,
            gauge::GaugePlugin,
            keypad::KeypadPlugin,
            knob::KnobPlugin,
            latch_button::LatchButtonPlugin,
            lcd::LcdPlugin,
            liquid::LiquidPlugin,
            paper::PaperPlugin,
            screen::ScreenPlugin,
            steam::SteamPlugin,
            sticker::StickerPlugin,
        ));
    }
}
