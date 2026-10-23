//! The rubber stamp and the service bell, for finishing a job.

use std::f32::consts::PI;

use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_core::audio::{Play, Sound};
use ld60_core::parts::paper::Paper;
use ld60_core::player::focus::{Clickable, Focus, FocusOn};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use super::job::{HandIn, Job};
use super::ticket::{JobTicket, Landing};

pub(super) const STAMP_BASE_SIZE: Vec3 = Vec3::new(0.06, 0.018, 0.04);
const STAMP_PAD_SIZE: Vec3 = Vec3::new(0.058, 0.005, 0.038);
const STAMP_NECK_SIZE: Vec3 = Vec3::new(0.012, 0.03, 0.012);
const STAMP_KNOB_SIZE: Vec3 = Vec3::new(0.03, 0.024, 0.03);
const STAMP_WOOD: Color = Color::srgb_u8(0x9a, 0x6e, 0x48);
const STAMP_RUBBER: Color = Color::srgb_u8(0x5a, 0x1c, 0x18);
const STAMP_HANDLE: Color = Color::srgb_u8(0xb0, 0x2e, 0x24);

/// How far the stamp moves down when pressed, and for how long.
const PRESS_DEPTH: f32 = 0.012;
const PRESS_TIME: f32 = 0.2;

pub(super) const BELL_BASE_SIZE: Vec3 = Vec3::new(0.07, 0.012, 0.07);
const BELL_DOME_RADIUS: f32 = 0.032;
/// The dome is a flattened sphere.
const BELL_DOME_FLATTEN: f32 = 0.75;
const BELL_PLUNGER_SIZE: Vec3 = Vec3::new(0.008, 0.012, 0.008);
const BELL_BASE: Color = Color::srgb_u8(0x22, 0x20, 0x1e);
const BRASS: Color = Color::srgb_u8(0xc8, 0x9a, 0x4a);

pub(crate) struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Press::animate)
            .add_observer(Tool::on_add)
            .add_observer(Tool::use_on_press);
    }
}

/// A tool on the counter that finishes a job. It can be used at any time.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(Transform, Visibility)]
pub(crate) enum Tool {
    /// Stamps the ticket of the job.
    Stamp,
    /// Calls the customer for a stamped job.
    Bell,
}

impl Tool {
    const fn prompt(self) -> &'static str {
        match self {
            Self::Stamp => "click-stamp",
            Self::Bell => "click-bell",
        }
    }

    fn use_on_press(
        mut press: On<PointerPress>,
        state: Res<State<Focus>>,
        tools: Query<(&Tool, &Transform, Has<Press>)>,
        landings: Query<(), With<Landing>>,
        job: Option<ResMut<Job>>,
        mut tickets: Query<(Entity, &mut Paper), With<JobTicket>>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || *state.get() != Focus::Desk {
            return;
        }
        let Ok((&tool, &rest, pressed)) = tools.get(press.entity) else {
            return;
        };
        press.propagate(false);
        match tool {
            Self::Stamp => {
                if !pressed {
                    commands
                        .entity(press.entity)
                        .insert(Press { elapsed: 0.0, rest });
                }
                match job {
                    Some(mut job) if !job.stamped => {
                        job.stamped = true;
                        if let Ok((ticket, mut paper)) = tickets.single_mut() {
                            paper.open_first_page();
                            commands.trigger(FocusOn { entity: ticket });
                        }
                    }
                    _ => commands.trigger(Play(Sound::Thunk)),
                }
            }
            Self::Bell => {
                commands.trigger(Play(Sound::Ding));
                if job.is_some_and(|job| job.stamped) && landings.is_empty() {
                    commands.trigger(HandIn);
                }
            }
        }
    }

    fn on_add(
        add: On<Add<Tool>>,
        tools: Query<&Tool>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok(&tool) = tools.get(add.entity) else {
            return;
        };
        let mut entity = commands.entity(add.entity);
        entity.insert(Clickable {
            prompt: tool.prompt(),
        });
        match tool {
            Self::Stamp => {
                let pad = Block::new(STAMP_PAD_SIZE);
                let base = Block::new(STAMP_BASE_SIZE).on_top_of(&pad);
                let neck = Block::new(STAMP_NECK_SIZE).on_top_of(&base);
                let knob = Block::new(STAMP_KNOB_SIZE).on_top_of(&neck);
                entity.with_children(|stamp| {
                    stamp.spawn((Shape::cuboid(&pad, STAMP_RUBBER), Finish::Plastic));
                    stamp.spawn((Shape::cuboid(&base, STAMP_WOOD), Finish::Wood));
                    stamp.spawn((Shape::cylinder(&neck, STAMP_WOOD), Finish::Wood));
                    stamp.spawn((Shape::cylinder(&knob, STAMP_HANDLE), Finish::Plastic));
                });
            }
            Self::Bell => {
                let base = Block::new(BELL_BASE_SIZE);
                let dome_height = BELL_DOME_RADIUS * BELL_DOME_FLATTEN;
                let plunger = Block::new(BELL_PLUNGER_SIZE)
                    .translated(Vec3::Y * (base.max().y + dome_height - 0.002));
                let brass = materials.add(StandardMaterial {
                    base_color: BRASS,
                    metallic: 1.0,
                    perceptual_roughness: 0.3,
                    ..default()
                });
                entity.with_children(|bell| {
                    bell.spawn((Shape::cylinder(&base, BELL_BASE), Finish::Wood));
                    bell.spawn((
                        Mesh3d(meshes.add(Sphere::new(BELL_DOME_RADIUS))),
                        MeshMaterial3d(brass.clone()),
                        Transform::from_translation(Vec3::Y * base.max().y).with_scale(Vec3::new(
                            1.0,
                            BELL_DOME_FLATTEN,
                            1.0,
                        )),
                    ));
                    bell.spawn((
                        Mesh3d(meshes.add(Cylinder::new(
                            BELL_PLUNGER_SIZE.x / 2.0,
                            BELL_PLUNGER_SIZE.y,
                        ))),
                        MeshMaterial3d(brass),
                        plunger.transform(),
                    ));
                });
            }
        }
    }
}

/// A tool moving down and back up once after a click.
#[derive(Component, Debug)]
struct Press {
    elapsed: f32,
    rest: Transform,
}

impl Press {
    fn animate(
        time: Res<Time>,
        mut tools: Query<(Entity, &mut Press, &mut Transform)>,
        mut commands: Commands,
    ) {
        for (tool, mut press, mut transform) in &mut tools {
            press.elapsed += time.delta_secs();
            let t = (press.elapsed / PRESS_TIME).min(1.0);
            *transform = press.rest;
            transform.translation.y -= PRESS_DEPTH * (PI * t).sin();
            if t >= 1.0 {
                commands.entity(tool).remove::<Press>();
            }
        }
    }
}
