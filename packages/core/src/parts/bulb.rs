use std::f32::consts::FRAC_PI_2;

use bevy::{light::NotShadowCaster, prelude::*};

/// Glass that lets most light through, frosted a little.
const GLASS_ALPHA: f32 = 0.2;
/// The filament is a ring this big, as a share of the bulb's radius.
const FILAMENT: f32 = 0.25;
const FILAMENT_WIRE: f32 = 0.033;
/// The filament glows this many times brighter than the glass around it.
const FILAMENT_GLOW: f32 = 65.0;
const TUNGSTEN: Color = Color::srgb_u8(0x30, 0x2c, 0x28);

pub struct BulbPlugin;

impl Plugin for BulbPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BulbMeshes>()
            .add_systems(Update, Bulb::show)
            .add_observer(Bulb::on_add);
    }
}

/// A light bulb: frosted glass around a glowing filament, centered on the
/// origin. It only glows, so it goes next to the light it stands for.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Visibility)]
pub struct Bulb {
    pub radius: f32,
    pub color: Color,
    /// How bright the glass glows while on.
    pub glow: f32,
    pub on: bool,
}

/// The materials of a [`Bulb`] that glow while it is on.
#[derive(Component, Debug)]
struct Glowing {
    glass: Handle<StandardMaterial>,
    filament: Handle<StandardMaterial>,
}

/// Meshes all bulbs share, for a radius of 1.
#[derive(Resource, Debug)]
struct BulbMeshes {
    glass: Handle<Mesh>,
    filament: Handle<Mesh>,
}

impl FromWorld for BulbMeshes {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        Self {
            glass: meshes.add(Sphere::new(1.0)),
            filament: meshes.add(Torus::new(
                FILAMENT - FILAMENT_WIRE,
                FILAMENT + FILAMENT_WIRE,
            )),
        }
    }
}

impl Bulb {
    fn on_add(
        add: On<Add<Bulb>>,
        bulbs: Query<&Bulb>,
        meshes: Res<BulbMeshes>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut commands: Commands,
    ) {
        let Ok(&bulb) = bulbs.get(add.entity) else {
            return;
        };
        let glowing = Glowing {
            glass: materials.add(StandardMaterial {
                base_color: bulb.color.with_alpha(GLASS_ALPHA),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.05,
                ..default()
            }),
            filament: materials.add(StandardMaterial {
                base_color: TUNGSTEN,
                metallic: 1.0,
                ..default()
            }),
        };
        let scale = Transform::from_scale(Vec3::splat(bulb.radius));
        commands
            .entity(add.entity)
            .with_child((
                Mesh3d(meshes.glass.clone()),
                MeshMaterial3d(glowing.glass.clone()),
                scale,
                NotShadowCaster,
            ))
            .with_child((
                Mesh3d(meshes.filament.clone()),
                MeshMaterial3d(glowing.filament.clone()),
                scale.with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                NotShadowCaster,
            ))
            .insert(glowing);
    }

    fn show(
        bulbs: Query<(&Bulb, &Glowing), Changed<Bulb>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for (bulb, glowing) in &bulbs {
            let glow = |strength: f32| {
                if bulb.on {
                    bulb.color.to_linear() * strength
                } else {
                    LinearRgba::BLACK
                }
            };
            if let Some(mut glass) = materials.get_mut(&glowing.glass) {
                glass.emissive = glow(bulb.glow);
            }
            if let Some(mut filament) = materials.get_mut(&glowing.filament) {
                filament.emissive = glow(bulb.glow * FILAMENT_GLOW);
            }
        }
    }
}
