mod editor;
mod keyboard;
mod monitor;
mod pinboard;
mod typing;
mod views;

use bevy::{mesh::MeshBuilder, prelude::*};
use ld60_game_core::state::AppState;
use ld60_game_manuals::Layouts;
use ld60_game_parts::dust::Dust;
use ld60_game_parts::keypad::PressableKey;
use ld60_game_parts::screen::{CrtGlass, CrtLight, CrtMaterial, ScreenSurface, UiCanvas};
use ld60_game_player::focus::{Focus, FocusTarget};
use ld60_game_player::hints::Hints;
use ld60_game_run::tutorial::FirstNote;
use ld60_geometry::block::Block;

use self::editor::Editor;
use self::keyboard::{Keyboard, KeyboardKey};
use self::monitor::Monitor;
use self::pinboard::PinBoard;
use self::pinboard::arm::Arm;
use self::pinboard::sway::Sway;
use self::typing::MouseTyping;
use self::views::{BackToScreen, Views};

const CASE: Color = Color::srgb_u8(0xd8, 0xcf, 0xb8);
const CASE_DARK: Color = Color::srgb_u8(0xb3, 0xa8, 0x8e);

/// Dust in front of the glass, which its light shows up in.
const DUST_DEPTH: f32 = 0.35;
/// The cloud is this much wider and taller than the glass.
const DUST_SPREAD: f32 = 1.3;
const DUST_MOTES: usize = 70;
const DUST_SEED: u64 = 13;

/// Space between the front of the monitor and the back of the keyboard.
const KEYBOARD_GAP: f32 = 0.04;
/// How much of the keyboard the wide view shows, from the back. The rest
/// would leave less of the window to the screen.
const KEYBOARD_SHOWN: f32 = 0.6;

pub struct ComputerPlugin;

impl Plugin for ComputerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((editor::EditorPlugin, views::ViewsPlugin))
            .add_systems(OnEnter(Focus::Computer), Computer::activate::<true>)
            .add_systems(
                OnExit(Focus::Computer),
                (Computer::activate::<false>, Editor::release_focus),
            )
            .init_resource::<MouseTyping>()
            .add_systems(
                Update,
                (
                    PinBoard::follow_pins.run_if(resource_exists::<Layouts>),
                    PinBoard::arm,
                    Arm::swing,
                    (Sway::tilt_new_pages, Sway::swing).chain(),
                    MouseTyping::release,
                ),
            )
            .add_systems(
                Update,
                PinBoard::show_first_note
                    .run_if(resource_exists::<FirstNote>)
                    .run_if(in_state(Focus::Desk))
                    .run_if(in_state(AppState::Playing)),
            )
            .add_observer(PinBoard::put_back_on_press)
            .add_observer(Arm::expect_pages)
            .add_observer(MouseTyping::press_on_click)
            .add_observer(Computer::on_add);
    }
}

/// A CRT monitor with a keyboard in front of it, resting on the ground.
/// Pinned sheets of the manuals file hang left of the monitor.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct Computer;

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
        );
        let views = layout.views();
        let (view, zoom, aside) = views.initial(&pins);
        commands
            .entity(add.entity)
            .insert((dust, pins, views, zoom, aside, Hints::default()))
            .insert(FocusTarget {
                prompt: "click-computer",
                focus: Focus::Computer,
                view,
            })
            .with_children(|parent| {
                let monitor = layout.monitor.spawn(
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
                parent.commands().entity(monitor).insert(BackToScreen);
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
pub struct ComputerLayout {
    monitor: Monitor,
    keyboard: Keyboard,
    /// Where the keyboard sits.
    pub keyboard_placement: Block,
}

impl Default for ComputerLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputerLayout {
    pub fn new() -> Self {
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

    pub fn monitor_body(&self) -> Block {
        self.monitor.body
    }

    /// The bar of the bezel below the glass.
    pub fn bezel_bottom(&self) -> Block {
        self.monitor.bezel[1]
    }

    /// The wide view shows the glass and the back of the keyboard. It holds
    /// on to the middle of the keyboard, so narrow windows cut off its sides
    /// and not the glass.
    fn views(&self) -> Views {
        let glass = self.monitor.glass;
        let corners = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
            .map(|(x, y)| glass.center + Vec3::new(x, y, 0.0) * glass.size / 2.0);
        let keyboard = self.keyboard_placement;
        let shown = keyboard.min().z + keyboard.size.z * KEYBOARD_SHOWN;
        let edge = Vec3::new(keyboard.center.x, keyboard.max().y, shown);
        Views::new(corners, [edge])
    }
}
