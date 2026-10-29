use std::f32::consts::FRAC_PI_2;

use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_geometry::block::Block;
use ld60_geometry::surface::{Finish, Form, SurfaceMaterial};

use crate::audio::{Play, Sound};
use crate::player::focus::Focused;

const CAP_RADIUS: f32 = 0.006;
/// Length of the cap's stem.
const CAP_HEIGHT: f32 = 0.008;
/// How far the cap sinks while latched down.
const TRAVEL: f32 = 0.004;
/// Height of the domed end of the cap.
const DOME: f32 = 0.0025;
/// The ring the cap moves in: its outer radius and height.
const COLLAR: Vec2 = Vec2::new(0.0095, 0.002);
/// Gap between the cap and the ring.
const PLAY: f32 = 0.0003;
/// Seconds the cap takes to move.
const MOVE_TIME: f32 = 0.05;

const COLLAR_COLOR: Color = Color::srgb_u8(0xb8, 0xbc, 0xc0);

pub struct LatchButtonPlugin;

impl Plugin for LatchButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, LatchButton::show)
            .add_observer(LatchButton::on_add)
            .add_observer(LatchButton::press);
    }
}

/// A round push button that latches: one press keeps it down, the next lets
/// it spring back up. It stands up along +Y from a collar at the origin and
/// works while the device it is on is focused.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct LatchButton {
    pub down: bool,
    pub color: Color,
}

/// The moving cap of a [`LatchButton`].
#[derive(Component, Debug)]
struct Cap;

impl LatchButton {
    fn on_add(
        add: On<Add<LatchButton>>,
        buttons: Query<&LatchButton>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let Ok(button) = buttons.get(add.entity) else {
            return;
        };
        let ring = Extrusion::new(Annulus::new(CAP_RADIUS + PLAY, COLLAR.x), COLLAR.y);
        let stem = Block::new(Vec3::new(2.0 * CAP_RADIUS, CAP_HEIGHT, 2.0 * CAP_RADIUS))
            .translated(-Vec3::Y * TRAVEL);
        let metal = surfaces.add(Finish::Plated.material(COLLAR_COLOR, Form::Freeform, 0.0));
        let plastic = surfaces.add(Finish::Plastic.material(button.color, Form::Freeform, 0.0));
        let dome = meshes.add(Sphere::new(CAP_RADIUS).mesh().uv(24, 12));
        let down = button.down;
        commands.entity(add.entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(ring)),
                MeshMaterial3d(metal),
                Transform::from_xyz(0.0, COLLAR.y / 2.0, 0.0)
                    .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
            ));
            parent
                .spawn((
                    Cap,
                    Transform::from_translation(Self::cap_at(down)),
                    Visibility::default(),
                ))
                .with_children(|cap| {
                    cap.spawn((
                        Mesh3d(meshes.add(Cylinder::new(CAP_RADIUS, CAP_HEIGHT))),
                        MeshMaterial3d(plastic.clone()),
                        stem.transform(),
                    ));
                    // A flattened dome on top of the stem.
                    cap.spawn((
                        Mesh3d(dome),
                        MeshMaterial3d(plastic),
                        Transform::from_translation(Vec3::Y * (CAP_HEIGHT - TRAVEL))
                            .with_scale(Vec3::new(1.0, DOME / CAP_RADIUS, 1.0)),
                    ));
                });
        });
    }

    fn cap_at(down: bool) -> Vec3 {
        if down { -Vec3::Y * TRAVEL } else { Vec3::ZERO }
    }

    fn press(
        mut press: On<PointerPress>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        mut buttons: Query<&mut LatchButton>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || !focused.holds(press.entity, &parents) {
            return;
        }
        let Ok(mut button) = buttons.get_mut(press.entity) else {
            return;
        };
        press.propagate(false);
        button.down = !button.down;
        commands.trigger(Play(if button.down {
            Sound::Latch
        } else {
            Sound::Unlatch
        }));
    }

    /// Moves the cap to where it rests, quickly.
    fn show(
        time: Res<Time>,
        buttons: Query<(&LatchButton, &Children)>,
        mut caps: Query<&mut Transform, With<Cap>>,
    ) {
        for (button, children) in &buttons {
            for &child in children {
                if let Ok(mut transform) = caps.get_mut(child) {
                    let goal = Self::cap_at(button.down);
                    let step = TRAVEL * time.delta_secs() / MOVE_TIME;
                    let moved = transform.translation.y
                        + (goal.y - transform.translation.y).clamp(-step, step);
                    if moved != transform.translation.y {
                        transform.translation.y = moved;
                    }
                }
            }
        }
    }
}
