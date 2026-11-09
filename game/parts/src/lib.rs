pub mod bulb;
pub mod dot_matrix;
pub mod downwash;
pub mod dust;
pub mod faceplate;
pub mod gauge;
pub mod keypad;
pub mod knob;
pub mod latch_button;
pub mod lcd;
pub mod led;
pub mod liquid;
pub mod marking;
pub mod paper;
pub mod pcb;
pub mod push_button;
pub mod screen;
pub mod script;
pub mod steam;
pub mod sticker;
pub mod toggle;

use bevy::prelude::*;

/// Reusable pieces for in-world objects.
pub struct PartsPlugin;

impl Plugin for PartsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            bulb::BulbPlugin,
            dot_matrix::DotMatrixPlugin,
            downwash::DownwashPlugin,
            dust::DustPlugin,
            gauge::GaugePlugin,
            keypad::KeypadPlugin,
            knob::KnobPlugin,
            latch_button::LatchButtonPlugin,
            lcd::LcdPlugin,
            liquid::LiquidPlugin,
            paper::PaperPlugin,
            pcb::PcbPlugin,
            screen::ScreenPlugin,
            steam::SteamPlugin,
            sticker::StickerPlugin,
        ))
        .add_plugins((
            led::LedPlugin,
            push_button::PushButtonPlugin,
            toggle::TogglePlugin,
        ));
    }
}
