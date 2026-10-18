mod backdrop;
mod frontage;

use std::ops::Range;

use bevy::{light::VolumetricLight, prelude::*, shape::Aabb3d};
use rand::{Rng, RngExt};

use super::buildings::{FacadeMaterial, Skyline};
use super::{Kit, glow, rng};
use crate::geometry::block::Block;
use crate::geometry::surface::{Finish, SurfaceMaterial};
use crate::parts::script::{GLYPH_SIZE, Word};

const SEED: u64 = 3;

const GLYPHS: Range<usize> = 2..7;
/// Tube thickness as a share of the grid step.
const TUBE: f32 = 0.22;
const TUBE_GLOW: f32 = 30.0;

/// Share of the panel height taken by the text.
const TEXT_SHARE: f32 = 0.6;
const PANEL_DEPTH: f32 = 0.12;
/// Thickness of the legs of signs on roofs.
const LEG: f32 = 0.2;
const FLICKER_CHANCE: f64 = 0.15;

const SPILL_INTENSITY: f32 = 60_000.0;
const SPILL_RANGE: f32 = 12.0;
/// Distance of the light in front of a sign.
const SPILL_DISTANCE: f32 = 1.2;

const PANEL: Color = Color::srgb_u8(0x14, 0x13, 0x18);
const NEON: [Color; 6] = [
    Color::srgb_u8(0xff, 0x2a, 0x6d),
    Color::srgb_u8(0x05, 0xd9, 0xe8),
    Color::srgb_u8(0xb1, 0x4a, 0xed),
    Color::srgb_u8(0x7d, 0xff, 0x5a),
    Color::srgb_u8(0xff, 0xb3, 0x2e),
    Color::srgb_u8(0xff, 0x4f, 0x2e),
];

pub(super) struct SignsPlugin;

impl Plugin for SignsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Sign::spawn_all.after(Skyline::spawn))
            .add_systems(Update, Flicker::update);
    }
}

/// The space taken up by every sign, which cables keep out of.
#[derive(Resource, Debug)]
pub(super) struct SignBounds(pub(super) Vec<Aabb3d>);

/// A neon sign. Its text faces local +Z, reading along +X.
pub(super) struct Sign {
    /// Center of the panel.
    transform: Transform,
    size: Vec2,
    /// Which of the [`NEON`] colors it glows in.
    neon: usize,
    /// Text on both faces, for signs that stick out from a wall.
    double_sided: bool,
    vertical: bool,
    spill: bool,
    /// A tube along the edge of the panel.
    framed: bool,
    /// Height of the legs it stands on, 0 for none.
    legs: f32,
    word: Word,
}

/// A sign tube with a bad contact.
#[derive(Component, Debug)]
struct Flicker {
    material: Handle<StandardMaterial>,
    color: LinearRgba,
    phase: f32,
}

impl Sign {
    pub(super) fn spawn_all(
        mut commands: Commands,
        kit: Res<Kit>,
        skyline: Res<Skyline>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
        mut facades: ResMut<Assets<FacadeMaterial>>,
    ) {
        let mut rng = rng(SEED);
        let panel = surfaces.add(Finish::Paint.scaled(PANEL));
        let tubes: Vec<_> = NEON
            .iter()
            .map(|&color| materials.add(glow(color, TUBE_GLOW)))
            .collect();

        let mut signs = Vec::new();
        for frontage in &skyline.front {
            let Some(mut facade) = facades.get_mut(&frontage.facade) else {
                continue;
            };
            signs.extend(Sign::on_frontage(&frontage.block, &mut facade, &mut rng));
        }

        for backdrop in &skyline.back {
            let Some(mut facade) = facades.get_mut(&backdrop.front.facade) else {
                continue;
            };
            signs.extend(Sign::on_backdrop(backdrop, &mut facade, &mut rng));
        }

        commands.insert_resource(SignBounds(signs.iter().map(Sign::bounds).collect()));
        for sign in signs {
            sign.spawn(
                &mut commands,
                &kit,
                &panel,
                &tubes,
                &mut materials,
                &mut rng,
            );
        }
    }

    /// Spawns the panel, its tubes, and its light, if it spills any.
    fn spawn(
        &self,
        commands: &mut Commands,
        kit: &Kit,
        panel: &Handle<SurfaceMaterial>,
        tubes: &[Handle<StandardMaterial>],
        materials: &mut Assets<StandardMaterial>,
        rng: &mut impl Rng,
    ) {
        let color = NEON[self.neon];
        let flicker = if rng.random_bool(FLICKER_CHANCE) {
            let handle = materials.add(glow(color, TUBE_GLOW));
            Some(Flicker {
                material: handle,
                color: color.to_linear() * TUBE_GLOW,
                phase: rng.random_range(0.0..100.0),
            })
        } else {
            None
        };
        let tube_material = flicker
            .as_ref()
            .map_or_else(|| tubes[self.neon].clone(), |f| f.material.clone());
        let mut entity = commands.spawn((self.transform, Visibility::default()));
        if let Some(flicker) = flicker {
            entity.insert(flicker);
        }
        entity.with_children(|parent| {
            let board = Block {
                center: Vec3::ZERO,
                size: self.size.extend(PANEL_DEPTH),
            };
            parent.spawn(kit.cuboid(&board, panel.clone()));
            if self.legs > 0.0 {
                for side in [-0.4, 0.4] {
                    let leg = Block {
                        center: Vec3::new(
                            side * self.size.x,
                            -(self.size.y + self.legs) / 2.0,
                            0.0,
                        ),
                        size: Vec3::new(LEG, self.legs, LEG),
                    };
                    parent.spawn(kit.cuboid(&leg, panel.clone()));
                }
            }
            let faces: &[f32] = if self.double_sided {
                &[1.0, -1.0]
            } else {
                &[1.0]
            };
            for &face in faces {
                let rotation = if face > 0.0 {
                    Quat::IDENTITY
                } else {
                    Quat::from_rotation_y(std::f32::consts::PI)
                };
                for tube in self.tubes() {
                    parent.spawn((
                        Mesh3d(kit.cube.clone()),
                        MeshMaterial3d(tube_material.clone()),
                        Transform::from_rotation(rotation) * tube.transform(PANEL_DEPTH / 2.0),
                    ));
                }
            }
            if self.spill {
                parent.spawn((
                    VolumetricLight,
                    PointLight {
                        color,
                        intensity: SPILL_INTENSITY,
                        range: SPILL_RANGE,
                        ..default()
                    },
                    Transform::from_xyz(0.0, -self.size.y / 2.0, SPILL_DISTANCE),
                ));
            }
        });
    }

    /// The space the sign takes up.
    fn bounds(&self) -> Aabb3d {
        let half = self.size.extend(PANEL_DEPTH) / 2.0;
        let corners = [-1.0, 1.0].into_iter().flat_map(|x| {
            [-1.0, 1.0]
                .into_iter()
                .flat_map(move |y| [-1.0, 1.0].map(move |z| half * Vec3::new(x, y, z)))
        });
        let points: Vec<_> = corners
            .map(|corner| self.transform.transform_point(corner))
            .collect();
        let min = points.iter().copied().reduce(Vec3::min).unwrap_or_default();
        let max = points.iter().copied().reduce(Vec3::max).unwrap_or_default();
        Aabb3d {
            min: min.into(),
            max: max.into(),
        }
    }

    /// Panel height for a vertical `word` of the given width.
    fn fit_vertical(word: &Word, width: f32) -> f32 {
        let glyphs = word.size(true);
        let step = width * TEXT_SHARE / glyphs.x;
        glyphs.y * step + width * (1.0 - TEXT_SHARE)
    }

    /// Panel size for a horizontal `word` of the given height.
    fn fit(word: &Word, height: f32, max_width: f32) -> Vec2 {
        let step = height * TEXT_SHARE / GLYPH_SIZE.y;
        let width = word.size(false).x * step + height * (1.0 - TEXT_SHARE);
        Vec2::new(width.min(max_width), height)
    }

    /// Tubes for the word, fitted into the panel.
    fn tubes(&self) -> Vec<Tube> {
        let mut tubes = Tube::lettering(&self.word, self.size, self.vertical);
        let thickness = tubes.first().map_or(0.0, |tube| tube.size.y);
        if self.framed {
            let inset = self.size / 2.0 - thickness * 1.5;
            let corners = [
                Vec2::new(-inset.x, -inset.y),
                Vec2::new(inset.x, -inset.y),
                Vec2::new(inset.x, inset.y),
                Vec2::new(-inset.x, inset.y),
            ];
            for (index, &a) in corners.iter().enumerate() {
                let b = corners[(index + 1) % corners.len()];
                tubes.push(Tube::between(a, b, thickness));
            }
        }
        tubes
    }
}

/// One straight piece of neon tube, or a stroke of painted letters.
pub(super) struct Tube {
    center: Vec3,
    size: Vec3,
    rotation: Quat,
}

impl Tube {
    /// The strokes of `word` fitted into a panel of `size`, centered on the
    /// origin and facing +Z.
    pub(super) fn lettering(word: &Word, size: Vec2, vertical: bool) -> Vec<Self> {
        let room = size * TEXT_SHARE / word.size(vertical);
        let step = room.x.min(room.y);
        let thickness = step * TUBE;
        word.strokes(vertical)
            .into_iter()
            .map(|(a, b)| Self::between(a * step, b * step, thickness))
            .collect()
    }

    /// Placement of a unit cube for this tube, `z` in front of the panel.
    pub(super) fn transform(&self, z: f32) -> Transform {
        Transform::from_translation(self.center.with_z(z + self.size.z / 2.0))
            .with_rotation(self.rotation)
            .with_scale(self.size)
    }

    fn between(a: Vec2, b: Vec2, thickness: f32) -> Self {
        let delta = b - a;
        Self {
            center: ((a + b) / 2.0).extend(0.0),
            size: Vec3::new(delta.length() + thickness, thickness, thickness),
            rotation: Quat::from_rotation_z(delta.y.atan2(delta.x)),
        }
    }
}

impl Flicker {
    fn update(
        time: Res<Time>,
        flickers: Query<&Flicker>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for flicker in &flickers {
            let Some(mut material) = materials.get_mut(&flicker.material) else {
                continue;
            };
            let t = time.elapsed_secs() * 7.0 + flicker.phase;
            // Mostly on, with short bursts of stutter.
            let burst = (t * 0.13).sin() > 0.85;
            let on = !burst || (t * 37.0).sin() > 0.0;
            material.emissive = flicker.color * if on { 1.0 } else { 0.05 };
        }
    }
}
