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

pub(crate) struct ComputerPlugin;

impl Plugin for ComputerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(editor::EditorPlugin)
            .add_systems(Update, KeyCap::follow_input)
            .add_observer(Computer::on_add);
    }
}

/// A CRT monitor with a keyboard in front of it, resting on the ground.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Computer;

impl Computer {
    fn on_add(
        add: On<Add<Computer>>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut crt_materials: ResMut<Assets<CrtMaterial>>,
    ) {
        let layout = ComputerLayout::new();
        let canvas = UiCanvas::spawn(&mut commands, &mut images, editor::CANVAS_SIZE);
        Editor::spawn(&mut commands, canvas.camera);
        let labels = UiCanvas::spawn(&mut commands, &mut images, keyboard::LABELS_SIZE);
        Keyboard::spawn_labels(&mut commands, labels.camera);

        let glass = CrtGlass {
            size: monitor::GLASS_SIZE,
            bulge: monitor::GLASS_BULGE,
        };
        commands.entity(add.entity).with_children(|parent| {
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

/// Geometry of a [`Computer`] in its own space.
struct ComputerLayout {
    monitor: Monitor,
    keyboard: Keyboard,
    /// Where the keyboard sits.
    keyboard_placement: Block,
}

impl ComputerLayout {
    fn new() -> Self {
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
}
