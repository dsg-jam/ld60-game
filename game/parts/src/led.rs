use bevy::{light::NotShadowCaster, prelude::*};

const RADIUS: f32 = 0.0025;
/// The cylinder below the dome.
const BODY: f32 = 0.002;
/// How bright a lit LED glows.
const GLOW: f32 = 6.0;
/// How much of its color an unlit LED shows.
const DARK: f32 = 0.25;

pub struct LedPlugin;

impl Plugin for LedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Led::show).add_observer(Led::on_add);
    }
}

/// A small round LED standing up along +Y from the panel at the origin.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Visibility)]
pub struct Led {
    pub color: Color,
    pub on: bool,
}

/// The material of a [`Led`], which glows while it is on.
#[derive(Component, Debug)]
struct Lens(Handle<StandardMaterial>);

impl Led {
    fn on_add(
        add: On<Add<Led>>,
        leds: Query<&Led>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut commands: Commands,
    ) {
        let Ok(&led) = leds.get(add.entity) else {
            return;
        };
        let lens = materials.add(Self::material(led));
        commands
            .entity(add.entity)
            .insert(Lens(lens.clone()))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(meshes.add(Cylinder::new(RADIUS, BODY))),
                    MeshMaterial3d(lens.clone()),
                    Transform::from_xyz(0.0, BODY / 2.0, 0.0),
                    NotShadowCaster,
                ));
                parent.spawn((
                    Mesh3d(meshes.add(Sphere::new(RADIUS).mesh().uv(16, 8))),
                    MeshMaterial3d(lens),
                    Transform::from_xyz(0.0, BODY, 0.0),
                    NotShadowCaster,
                ));
            });
    }

    fn material(led: Led) -> StandardMaterial {
        let linear = led.color.to_linear();
        StandardMaterial {
            base_color: if led.on {
                led.color
            } else {
                (linear * DARK).with_alpha(1.0).into()
            },
            emissive: if led.on {
                linear * GLOW
            } else {
                LinearRgba::BLACK
            },
            perceptual_roughness: 0.15,
            ..default()
        }
    }

    fn show(
        leds: Query<(&Led, &Lens), Changed<Led>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for (&led, lens) in &leds {
            if let Some(mut material) = materials.get_mut(&lens.0) {
                *material = Self::material(led);
            }
        }
    }
}
