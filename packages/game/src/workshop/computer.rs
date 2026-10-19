mod editor;
mod keyboard;
mod monitor;
mod pinboard;

use bevy::{mesh::MeshBuilder, prelude::*};

use self::editor::Editor;
use self::keyboard::{Keyboard, KeyboardKey};
use self::monitor::Monitor;
use self::pinboard::PinBoard;
use super::dust::Dust;
use crate::geometry::block::Block;
use crate::parts::keypad::PressableKey;
use crate::parts::screen::{CrtGlass, CrtLight, CrtMaterial, ScreenSurface, UiCanvas};
use crate::player::focus::{Aside, Focus, FocusTarget, View};

const CASE: Color = Color::srgb_u8(0xd8, 0xcf, 0xb8);
const CASE_DARK: Color = Color::srgb_u8(0xb3, 0xa8, 0x8e);

/// Dust in front of the glass, which its light shows up in.
const DUST_DEPTH: f32 = 0.35;
/// The cloud is this much wider and taller than the glass.
const DUST_SPREAD: f32 = 1.3;
const DUST_MOTES: usize = 70;
const DUST_SEED: u64 = 13;

/// Space between the front of the monitor and the back of the keyboard.
const KEYBOARD_GAP: f32 = 0.06;
/// Distance of the camera from the glass while focused on the computer.
const SCREEN_VIEW_DISTANCE: f32 = 0.32;

pub(crate) struct ComputerPlugin;

impl Plugin for ComputerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(editor::EditorPlugin)
            .add_systems(OnEnter(Focus::Computer), Computer::activate::<true>)
            .add_systems(
                OnExit(Focus::Computer),
                (Computer::activate::<false>, Editor::release_focus),
            )
            .add_systems(Update, PinBoard::follow_pins)
            .add_observer(Computer::on_add);
    }
}

/// A CRT monitor with a keyboard in front of it, resting on the ground.
/// Pinned sheets of the manuals file hang left of the monitor.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Computer;

impl Computer {
    /// The screen and the keyboard only react while the player sits at the
    /// computer.
    fn activate<const ACTIVE: bool>(
        mut screens: Query<&mut ScreenSurface, With<ComputerScreen>>,
        mut keys: Query<&mut PressableKey, With<KeyboardKey>>,
    ) {
        for mut screen in &mut screens {
            screen.active = ACTIVE;
        }
        for mut key in &mut keys {
            key.active = ACTIVE;
        }
    }

    fn on_add(
        add: On<Add<Computer>>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut crt_materials: ResMut<Assets<CrtMaterial>>,
    ) {
        let layout = ComputerLayout::new();
        let canvas = UiCanvas::spawn(&mut commands, &mut images, editor::CANVAS_SIZE, add.entity);
        Editor::spawn(&mut commands, &canvas);
        let labels = layout
            .keyboard
            .spawn_labels(&mut commands, &mut images, add.entity);

        let glass = CrtGlass {
            size: monitor::GLASS_SIZE,
            bulge: monitor::GLASS_BULGE,
        };
        let screen = layout.monitor.glass;
        let dust = Dust {
            cloud: Block::new((screen.size.truncate() * DUST_SPREAD).extend(DUST_DEPTH))
                .centered_on(&screen)
                .translated(Vec3::Z * DUST_DEPTH / 2.0),
            motes: DUST_MOTES,
            seed: DUST_SEED,
        };
        let pins = PinBoard::spawn(
            &mut commands,
            add.entity,
            &layout.monitor,
            &mut images,
            &mut meshes,
            &mut materials,
        );
        commands
            .entity(add.entity)
            .insert((dust, pins, Aside::new(layout.screen_view())))
            .insert(FocusTarget {
                prompt: "click-computer",
                focus: Focus::Computer,
                view: View::Local(layout.screen_view()),
            })
            .with_children(|parent| {
                layout.monitor.spawn(
                    parent,
                    Transform::default(),
                    (
                        ComputerScreen,
                        ScreenSurface::new(&canvas),
                        Mesh3d(meshes.add(glass.build())),
                        MeshMaterial3d(crt_materials.add(CrtMaterial::new(&canvas))),
                        CrtLight {
                            size: monitor::GLASS_SIZE,
                        },
                    ),
                );
                layout.keyboard.spawn(
                    parent,
                    Transform::from_translation(layout.keyboard_placement.bottom()),
                    labels,
                );
            });
    }
}

/// The glass of the computer's monitor.
#[derive(Component)]
struct ComputerScreen;

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
        let keyboard = Keyboard::new();
        let keyboard_placement = Block::new(keyboard.base.size)
            .in_front_of(&monitor.body)
            .translated(Vec3::Z * KEYBOARD_GAP);
        Self {
            monitor,
            keyboard,
            keyboard_placement,
        }
    }

    /// Camera pose straight in front of the glass.
    fn screen_view(&self) -> Transform {
        let glass = self.monitor.glass.center;
        Transform::from_translation(glass + Vec3::Z * SCREEN_VIEW_DISTANCE)
            .looking_at(glass, Vec3::Y)
    }
}
