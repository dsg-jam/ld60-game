pub(crate) mod keypad;
pub(crate) mod lcd;
pub(crate) mod paper;
pub(crate) mod screen;
pub(crate) mod sticker;

use bevy::prelude::*;

/// Reusable pieces for in-world objects.
pub(crate) struct PartsPlugin;

impl Plugin for PartsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            keypad::KeypadPlugin,
            lcd::LcdPlugin,
            paper::PaperPlugin,
            screen::ScreenPlugin,
            sticker::StickerPlugin,
        ));
    }
}
