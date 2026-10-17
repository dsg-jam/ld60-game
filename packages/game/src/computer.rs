mod editor;
mod keyboard;
mod monitor;

use bevy::{mesh::MeshBuilder, prelude::*};

use self::editor::Editor;
use self::keyboard::{KeyCap, Keyboard};
use self::monitor::Monitor;
use crate::layout::Block;
use crate::screen::{CrtGlass, CrtMaterial, ScreenSurface, UiCanvas};

const CASE: Color = Color::srgb_u8(0xd8, 0xcf, 0xb8);
const CASE_DARK: Color = Color::srgb_u8(0xb3, 0xa8, 0x8e);

/// Space between the front of the monitor and the back of the keyboard.
const KEYBOARD_GAP: f32 = 0.06;
/// Distance of the overview camera from the glass.
const VIEW_DISTANCE: f32 = 0.75;
/// Height of the overview camera above the screen center.
const VIEW_RAISE: f32 = 0.15;
/// How far the overview camera looks from the screen towards the keyboard,
/// from 0 (screen) to 1 (keyboard).
const VIEW_KEYBOARD_BIAS: f32 = 0.3;

pub(crate) struct ComputerPlugin;

impl Plugin for ComputerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(editor::EditorPlugin)
            .add_systems(Startup, Computer::spawn)
            .add_systems(Update, KeyCap::follow_input);
    }
}

/// A CRT monitor with a keyboard in front of it, resting on the ground.
pub(crate) struct Computer {
    monitor: Monitor,
    keyboard: Keyboard,
    /// Where the keyboard sits, in the computer's space.
    keyboard_placement: Block,
}

impl Computer {
    pub(crate) fn new() -> Self {
        let monitor = Monitor::new();
        let keyboard_placement = Block::new(keyboard::BASE_SIZE)
            .in_front_of(&monitor.body)
            .translated(Vec3::Z * KEYBOARD_GAP);
        Self {
            monitor,
            keyboard: Keyboard::new(),
            keyboard_placement,
        }
    }

    /// Camera pose that shows the screen and the keyboard.
    pub(crate) fn overview(&self) -> Transform {
        let screen = self.monitor.glass.center;
        let target = screen.lerp(self.keyboard_placement.center, VIEW_KEYBOARD_BIAS);
        Transform::from_translation(screen + Vec3::new(0.0, VIEW_RAISE, VIEW_DISTANCE))
            .looking_at(target, Vec3::Y)
    }

    fn spawn(
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut crt_materials: ResMut<Assets<CrtMaterial>>,
    ) {
        let layout = Self::new();
        let canvas = UiCanvas::spawn(&mut commands, &mut images, editor::CANVAS_SIZE);
        Editor::spawn(&mut commands, canvas.camera);
        let labels = UiCanvas::spawn(&mut commands, &mut images, keyboard::LABELS_SIZE);
        Keyboard::spawn_labels(&mut commands, labels.camera);

        let glass = CrtGlass {
            size: monitor::GLASS_SIZE,
            bulge: monitor::GLASS_BULGE,
        };
        commands
            .spawn((
                Name::new("computer"),
                Transform::default(),
                Visibility::default(),
            ))
            .with_children(|parent| {
                layout.monitor.spawn(
                    parent,
                    Transform::default(),
                    (
                        ScreenSurface::new(&canvas),
                        Mesh3d(meshes.add(glass.build())),
                        MeshMaterial3d(crt_materials.add(CrtMaterial::new(&canvas))),
                    ),
                );
                layout.keyboard.spawn(
                    parent,
                    Transform::from_translation(layout.keyboard_placement.bottom()),
                    labels.image,
                );
            });
    }
}
