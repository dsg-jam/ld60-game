mod keyboard;

use bevy::prelude::*;

use self::keyboard::{KeyCap, Keyboard};
use crate::layout::Block;
use crate::screen::UiCanvas;

const CASE: Color = Color::srgb_u8(0xd8, 0xcf, 0xb8);
const CASE_DARK: Color = Color::srgb_u8(0xb3, 0xa8, 0x8e);

/// Offset of the overview camera from the keyboard center.
const VIEW_OFFSET: Vec3 = Vec3::new(0.0, 0.25, 0.3);

pub(crate) struct ComputerPlugin;

impl Plugin for ComputerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Computer::spawn)
            .add_systems(Update, KeyCap::follow_input);
    }
}

/// The computer, resting on the ground. For now just its keyboard.
pub(crate) struct Computer {
    keyboard: Keyboard,
    /// Where the keyboard sits, in the computer's space.
    keyboard_placement: Block,
}

impl Computer {
    pub(crate) fn new() -> Self {
        Self {
            keyboard: Keyboard::new(),
            keyboard_placement: Block::new(keyboard::BASE_SIZE),
        }
    }

    /// Camera pose that shows the keyboard.
    pub(crate) fn overview(&self) -> Transform {
        let target = self.keyboard_placement.center;
        Transform::from_translation(target + VIEW_OFFSET).looking_at(target, Vec3::Y)
    }

    fn spawn(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
        let layout = Self::new();
        let labels = UiCanvas::spawn(&mut commands, &mut images, keyboard::LABELS_SIZE);
        Keyboard::spawn_labels(&mut commands, labels.camera);
        commands
            .spawn((Transform::default(), Visibility::default()))
            .with_children(|parent| {
                layout.keyboard.spawn(
                    parent,
                    Transform::from_translation(layout.keyboard_placement.bottom()),
                    labels.image,
                );
            });
    }
}
