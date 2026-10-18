//! The rubber stamp and the service bell, for finishing a job.

use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};

use super::job::{HandIn, Job};
use super::ticket::JobTicket;
use crate::audio::{Play, Sound};
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::player::focus::{Clickable, Focus, FocusOn};

const STAMP_BASE_SIZE: Vec3 = Vec3::new(0.06, 0.018, 0.04);
const STAMP_PAD_SIZE: Vec3 = Vec3::new(0.058, 0.005, 0.038);
const STAMP_NECK_SIZE: Vec3 = Vec3::new(0.012, 0.03, 0.012);
const STAMP_KNOB_SIZE: Vec3 = Vec3::new(0.03, 0.024, 0.03);
const STAMP_WOOD: Color = Color::srgb_u8(0x9a, 0x6e, 0x48);
const STAMP_RUBBER: Color = Color::srgb_u8(0x5a, 0x1c, 0x18);
const STAMP_HANDLE: Color = Color::srgb_u8(0xb0, 0x2e, 0x24);

const BELL_BASE_SIZE: Vec3 = Vec3::new(0.07, 0.012, 0.07);
const BELL_DOME_RADIUS: f32 = 0.032;
/// The dome is a flattened sphere.
const BELL_DOME_FLATTEN: f32 = 0.75;
const BELL_PLUNGER_SIZE: Vec3 = Vec3::new(0.008, 0.012, 0.008);
const BELL_BASE: Color = Color::srgb_u8(0x22, 0x20, 0x1e);
const BRASS: Color = Color::srgb_u8(0xc8, 0x9a, 0x4a);

pub(crate) struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Tool::arm)
            .add_observer(Tool::on_add)
            .add_observer(Tool::use_on_press);
    }
}

/// A tool on the desk that finishes a job. It is only clickable when it is
/// the next step.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(Transform, Visibility)]
pub(crate) enum Tool {
    /// Stamps the ticket of a repaired job.
    Stamp,
    /// Calls the customer for a stamped job.
    Bell,
}

impl Tool {
    fn usable(self, job: Option<&Job>) -> bool {
        match (self, job) {
            (Self::Stamp, Some(job)) => job.repaired && !job.stamped,
            (Self::Bell, Some(job)) => job.stamped,
            (_, None) => false,
        }
    }

    fn arm(
        job: Option<Res<Job>>,
        tools: Query<(Entity, &Tool, Has<Clickable>)>,
        mut commands: Commands,
    ) {
        for (entity, tool, clickable) in &tools {
            let usable = tool.usable(job.as_deref());
            if usable && !clickable {
                commands.entity(entity).insert(Clickable);
            } else if !usable && clickable {
                commands.entity(entity).remove::<Clickable>();
            }
        }
    }

    fn use_on_press(
        mut press: On<PointerPress>,
        state: Res<State<Focus>>,
        tools: Query<&Tool, With<Clickable>>,
        job: Option<ResMut<Job>>,
        tickets: Query<Entity, With<JobTicket>>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || *state.get() != Focus::Desk {
            return;
        }
        let (Ok(&tool), Some(mut job)) = (tools.get(press.entity), job) else {
            return;
        };
        press.propagate(false);
        match tool {
            Self::Stamp => {
                job.stamped = true;
                commands.trigger(Play(Sound::Thunk));
                if let Ok(ticket) = tickets.single() {
                    commands.trigger(FocusOn { entity: ticket });
                }
            }
            Self::Bell => {
                commands.trigger(Play(Sound::Ding));
                commands.trigger(HandIn);
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
        match tool {
            Self::Stamp => {
                let pad = Block::new(STAMP_PAD_SIZE);
                let base = Block::new(STAMP_BASE_SIZE).on_top_of(&pad);
                let neck = Block::new(STAMP_NECK_SIZE).on_top_of(&base);
                let knob = Block::new(STAMP_KNOB_SIZE).on_top_of(&neck);
                entity.with_children(|stamp| {
                    stamp.spawn(Shape::cuboid(&pad, STAMP_RUBBER));
                    stamp.spawn(Shape::cuboid(&base, STAMP_WOOD));
                    stamp.spawn(Shape::cylinder(&neck, STAMP_WOOD));
                    stamp.spawn(Shape::cylinder(&knob, STAMP_HANDLE));
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
                    bell.spawn(Shape::cylinder(&base, BELL_BASE));
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
