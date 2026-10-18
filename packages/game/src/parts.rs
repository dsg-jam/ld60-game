pub(crate) mod dot_matrix;
pub(crate) mod gauge;
pub(crate) mod keypad;
pub(crate) mod knob;
pub(crate) mod latch_button;
pub(crate) mod lcd;
pub(crate) mod liquid;
pub(crate) mod paper;
pub(crate) mod screen;
pub(crate) mod script;
pub(crate) mod sticker;

use bevy::prelude::*;

/// Reusable pieces for in-world objects.
pub(crate) struct PartsPlugin;

impl Plugin for PartsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            dot_matrix::DotMatrixPlugin,
            gauge::GaugePlugin,
            keypad::KeypadPlugin,
            knob::KnobPlugin,
            latch_button::LatchButtonPlugin,
            lcd::LcdPlugin,
            liquid::LiquidPlugin,
            paper::PaperPlugin,
            screen::ScreenPlugin,
            sticker::StickerPlugin,
        ));
    }
}
