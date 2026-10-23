use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

/// The pointer turns on a pivot this far below the scale, hidden behind
/// whatever the gauge is mounted in.
const PIVOT_DROP: f32 = 0.03;
const POINTER_WIDTH: f32 = 0.002;
/// The pointer reaches to this far below the top of the scale.
const POINTER_GAP: f32 = 0.004;
/// From the first mark to the last, the pointer turns this far.
const SWEEP: f32 = 1.4;
const TICK: Vec2 = Vec2::new(0.0015, 0.006);
/// The marks sit this far below where the pointer ends.
const TICK_INSET: f32 = 0.004;
/// Space between the scale and the pointer, and the pointer's thickness.
const LAYER: f32 = 0.001;
const GLOW: f32 = 3.0;

const AMBER: Color = Color::srgb_u8(0xff, 0xb0, 0x50);
const DARK: Color = Color::srgb_u8(0x30, 0x22, 0x18);
const RED: Color = Color::srgb_u8(0xd0, 0x2a, 0x1a);

pub(crate) struct GaugePlugin;

impl Plugin for GaugePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Gauge::show)
            .add_observer(Gauge::on_add);
    }
}

/// A backlit scale with marks on an arc and a pointer that turns to show a
/// value. The scale faces +Z with its middle at the origin, and the pointer
/// turns in front of it on a pivot below. Mount it behind a window, so the
/// pivot is hidden.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Gauge {
    /// Width and height of the scale. Only read when the gauge is added.
    pub(crate) size: Vec2,
    /// Marks on the scale, evenly spread. Only read when the gauge is added.
    pub(crate) marks: usize,
    /// From 0 at the first mark to 1 at the last.
    pub(crate) value: f32,
    pub(crate) lit: bool,
}

/// The pointer of a [`Gauge`].
#[derive(Component, Debug)]
struct Pointer;

/// The backlit scale of a [`Gauge`].
#[derive(Component, Debug)]
struct Scale;

impl Gauge {
    /// The value of mark `index`.
    pub(crate) fn mark(&self, index: usize) -> f32 {
        index as f32 / (self.marks.max(2) - 1) as f32
    }

    /// Turn of the pointer at `value`, counterclockwise from straight up.
    fn angle(value: f32) -> f32 {
        SWEEP * (0.5 - value)
    }

    fn on_add(
        add: On<Add<Gauge>>,
        gauges: Query<&Gauge>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok(gauge) = gauges.get(add.entity) else {
            return;
        };
        let pivot = Vec3::new(0.0, -gauge.size.y / 2.0 - PIVOT_DROP, LAYER);
        let reach = gauge.size.y + PIVOT_DROP - POINTER_GAP;
        let pointer = Block::new(Vec3::new(POINTER_WIDTH, reach, LAYER));
        let marks: Vec<_> = (0..gauge.marks).map(|index| gauge.mark(index)).collect();
        let light = materials.add(StandardMaterial {
            base_color: AMBER,
            emissive: AMBER.to_linear() * GLOW,
            ..default()
        });
        commands.entity(add.entity).with_children(|parent| {
            parent.spawn((
                Scale,
                Mesh3d(meshes.add(Rectangle::from_size(gauge.size))),
                MeshMaterial3d(light),
            ));
            for value in marks {
                let turn = Quat::from_rotation_z(Self::angle(value));
                let at = pivot + turn * Vec3::Y * (reach - TICK_INSET - TICK.y / 2.0);
                parent
                    .spawn(Shape::cuboid(&Block::new(TICK.extend(LAYER / 2.0)), DARK))
                    .insert(
                        Transform::from_translation(at.with_z(LAYER / 4.0)).with_rotation(turn),
                    );
            }
            parent
                .spawn((
                    Pointer,
                    Transform::from_translation(pivot),
                    Visibility::default(),
                ))
                .with_child((Shape::cuboid(&pointer, RED), Finish::Plastic));
        });
    }

    fn show(
        gauges: Query<(&Gauge, &Children), Changed<Gauge>>,
        mut pointers: Query<&mut Transform, With<Pointer>>,
        scales: Query<&MeshMaterial3d<StandardMaterial>, With<Scale>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for (gauge, children) in &gauges {
            for &child in children {
                if let Ok(mut transform) = pointers.get_mut(child) {
                    transform.rotation = Quat::from_rotation_z(Self::angle(gauge.value));
                }
                if let Ok(scale) = scales.get(child)
                    && let Some(mut material) = materials.get_mut(scale)
                {
                    let glow = if gauge.lit { GLOW } else { 0.0 };
                    material.emissive = AMBER.to_linear() * glow;
                    material.base_color = if gauge.lit { AMBER } else { DARK };
                }
            }
        }
    }
}
