use bevy::prelude::*;
#[cfg(feature = "debug_port")]
use ld60_game_core::debug_port::{DebugMethod, DebugMethods};
use ld60_geometry::cloth::{self, Drape};
use ld60_geometry::surface::{Finish, Form, SurfaceMaterial};

use ld60_game_parts::screen::CrtLight;

use ld60_game_player::focus::Disabled;

use super::monitor::Monitor;
use super::{Computer, ComputerScreen};

/// The cloth stands this far off the monitor.
const CLEARANCE: f32 = 0.004;
const DRAPE: Drape = Drape {
    top: Rect {
        min: Vec2::ZERO,
        max: Vec2::ZERO,
    },
    height: 0.0,
    pool: -0.02,
    roll: 0.015,
    folds: Vec2::new(0.02, 0.08),
    cells: 96,
};
/// Old linen, grey with dust.
const LINEN: Color = Color::srgb_u8(0xc8, 0xc4, 0xb8);

pub(super) struct CoverPlugin;

impl Plugin for CoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(DustCover::on_add)
            .add_observer(DustCover::darken)
            .add_observer(DustCover::lighten);
        #[cfg(feature = "debug_port")]
        app.add_debug_method(
            DebugMethod::new(
                "ld60.cover",
                "Throws a dust sheet over the monitor, or takes it off again.",
            ),
            DustCover::debug_toggle,
        );
    }
}

/// A dust sheet thrown over the monitor of a [`Computer`](super::Computer)
/// standing at the same place. It is not part of the computer, so clicks
/// on it don't reach the computer.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct DustCover;

impl DustCover {
    fn on_add(
        add: On<Add<DustCover>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
        mut commands: Commands,
    ) {
        let (top, height) = Monitor::new().outline();
        let drape = Drape {
            top: Rect::from_corners(
                top.min - Vec2::splat(CLEARANCE),
                top.max + Vec2::splat(CLEARANCE),
            ),
            height: height + CLEARANCE,
            ..DRAPE
        };
        let material = Finish::Fabric.over(
            StandardMaterial {
                base_color: LINEN,
                perceptual_roughness: 1.0,
                double_sided: true,
                cull_mode: None,
                ..default()
            },
            Form::Freeform,
            0.3,
        );
        commands.entity(add.entity).with_child((
            Mesh3d(meshes.add(cloth::drape(&drape))),
            MeshMaterial3d(surfaces.add(material)),
        ));
    }

    /// The screen under a cover casts no light, and the computer can't be
    /// used.
    fn darken(
        _: On<Add<DustCover>>,
        mut glasses: Query<&mut CrtLight, With<ComputerScreen>>,
        computers: Query<Entity, With<Computer>>,
        mut commands: Commands,
    ) {
        for mut glass in &mut glasses {
            glass.dark = true;
        }
        for computer in &computers {
            commands.entity(computer).insert(Disabled);
        }
    }

    fn lighten(
        _: On<Remove<DustCover>>,
        mut glasses: Query<&mut CrtLight, With<ComputerScreen>>,
        computers: Query<Entity, With<Computer>>,
        mut commands: Commands,
    ) {
        for mut glass in &mut glasses {
            glass.dark = false;
        }
        for computer in &computers {
            commands.entity(computer).remove::<Disabled>();
        }
    }

    #[cfg(feature = "debug_port")]
    fn debug_toggle(
        In(_): In<Option<serde_json::Value>>,
        covers: Query<Entity, With<DustCover>>,
        computers: Query<&Transform, With<Computer>>,
        mut commands: Commands,
    ) -> bevy::remote::BrpResult {
        let mut removed = false;
        for cover in &covers {
            commands.entity(cover).despawn();
            removed = true;
        }
        if !removed {
            for &placed in &computers {
                commands.spawn((Name::new("dust cover"), DustCover, placed));
            }
        }
        Ok(serde_json::Value::Bool(!removed))
    }
}
