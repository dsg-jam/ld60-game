mod candle;
mod sign;

use bevy::prelude::*;
use ld60_game_core::layout::shop::{FRONT, PILLAR, SIZE};
use ld60_game_core::parts::script::ALPHABET;
use ld60_geometry::bar::Bar;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use self::candle::{Candle, Paper};

/// Box of the rolled up shutter above the opening.
const SHUTTER_HEIGHT: f32 = 0.3;
/// The roof reaches out over the counter and slopes down. Short and high,
/// so it doesn't hide the skyline.
const ROOF_REACH: f32 = 0.8;
const ROOF_DROP: f32 = 0.2;
const ROOF_THICKNESS: f32 = 0.03;
const ROOF_RIBS: usize = 9;
/// Width and height of the ribs on the roof.
const RIB: Vec2 = Vec2::new(0.03, 0.02);

/// Bottom of the beam the noren hang from.
const BEAM_HEIGHT: f32 = 2.95;
const BEAM: Vec2 = Vec2::new(0.1, 0.08);
const NOREN_LENGTH: f32 = 0.4;
const NOREN_GAP: f32 = 0.03;
const NOREN_THICKNESS: f32 = 0.01;
/// The name of the shop, one letter on each noren.
const NAME: [usize; 3] = [4, 11, 17];
/// Size of one grid step of the letters on the noren.
const LETTER_STEP: f32 = 0.075;
const STROKE: f32 = 0.03;
/// How far the letters stand out of the fabric.
const LETTER_DEPTH: f32 = 0.004;

const LANTERN: Vec3 = Vec3::new(0.24, 0.32, 0.24);
/// How far out the lanterns hang, as a share of the roof's reach.
const LANTERN_OUT: f32 = 0.75;
const LANTERN_CORD: f32 = 0.2;
const LANTERN_GLOW: f32 = 6.0;
const LANTERN_INTENSITY: f32 = 2_500.0;
const LANTERN_RANGE: f32 = 4.0;

const STEEL: Color = Color::srgb_u8(0x3c, 0x40, 0x44);
const ROOF_METAL: Color = Color::srgb_u8(0x6a, 0x5a, 0x4a);
const BEAM_WOOD: Color = Color::srgb_u8(0x4a, 0x34, 0x24);
const NOREN: Color = Color::srgb_u8(0x2a, 0x35, 0x50);
const LETTERS: Color = Color::srgb_u8(0xe8, 0xe0, 0xcc);
const LETTERS_INSIDE: Color = Color::srgb_u8(0x6a, 0x6e, 0x78);
const LANTERN_PAPER: Color = Color::srgb_u8(0xff, 0x7a, 0x3a);
const CORD: Color = Color::srgb_u8(0x18, 0x16, 0x14);

pub(crate) struct FrontPlugin;

impl Plugin for FrontPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((candle::CandlePlugin, sign::SignPlugin))
            .add_observer(Front::on_add);
    }
}

/// The front of the shop like a yatai: a small roof over the counter,
/// noren with the shop's name and paper lanterns.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Front;

impl Front {
    fn on_add(
        add: On<Add<Front>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let half = SIZE.x / 2.0 - PILLAR.x;
        let shutter = Block::between(
            Vec3::new(-half, SIZE.y - SHUTTER_HEIGHT, FRONT),
            Vec3::new(half, SIZE.y, FRONT + PILLAR.y),
        );
        let beam = Block::between(
            Vec3::new(-half, BEAM_HEIGHT, FRONT + PILLAR.y / 2.0 - BEAM.y / 2.0),
            Vec3::new(
                half,
                BEAM_HEIGHT + BEAM.x,
                FRONT + PILLAR.y / 2.0 + BEAM.y / 2.0,
            ),
        );
        let roof_top = shutter.max().y;
        let lantern_mesh = meshes.add(Cylinder::new(LANTERN.x / 2.0, LANTERN.y));

        commands.entity(add.entity).with_children(|front| {
            front.spawn((Shape::cuboid(&shutter, STEEL), Finish::Metal));
            front.spawn((Shape::cuboid(&beam, BEAM_WOOD), Finish::Wood));
            Self::spawn_roof(front, roof_top);
            Self::spawn_noren(front, &beam);

            for (side, seed) in [(-1.0, 1), (1.0, 7)] {
                let out = ROOF_REACH * LANTERN_OUT;
                let under_roof = roof_top - ROOF_DROP * LANTERN_OUT;
                // Over the pillars, out of the way of the view.
                let x = side * (half + PILLAR.x / 2.0);
                let top = Vec3::new(x, under_roof, FRONT - out);
                let center = top - Vec3::Y * (LANTERN_CORD + LANTERN.y / 2.0);
                let cord = Block::between(
                    top - Vec3::new(0.005, LANTERN_CORD, 0.005),
                    top + Vec3::new(0.005, 0.0, 0.005),
                );
                front.spawn((Shape::cuboid(&cord, CORD), Finish::Fabric));
                let flame = center - Vec3::Y * LANTERN.y;
                let candle = front
                    .spawn((
                        PointLight {
                            color: LANTERN_PAPER,
                            intensity: LANTERN_INTENSITY,
                            range: LANTERN_RANGE,
                            ..default()
                        },
                        Transform::from_translation(flame),
                        Candle::new(LANTERN_PAPER, LANTERN_INTENSITY, flame, seed),
                    ))
                    .id();
                front.spawn((
                    Mesh3d(lantern_mesh.clone()),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: LANTERN_PAPER,
                        emissive: LANTERN_PAPER.to_linear() * LANTERN_GLOW,
                        ..default()
                    })),
                    Transform::from_translation(center),
                    Paper {
                        candle,
                        glow: LANTERN_GLOW,
                    },
                ));
            }
        });
    }

    /// Corrugated metal sloping down towards the street.
    fn spawn_roof(front: &mut ChildSpawnerCommands, top: f32) {
        let width = SIZE.x - 0.1;
        let sheet = Bar {
            from: Vec3::new(0.0, top, FRONT),
            to: Vec3::new(0.0, top - ROOF_DROP, FRONT - ROOF_REACH),
            section: Vec2::new(width, ROOF_THICKNESS),
            up: Vec3::Y,
        };
        front.spawn((Shape::bar(&sheet, ROOF_METAL), Finish::Rust));
        let tilt = sheet.placement().rotation;
        for index in 0..ROOF_RIBS {
            let x = -width / 2.0 + (index as f32 + 0.5) * width / ROOF_RIBS as f32;
            let rib = Bar {
                section: RIB,
                ..sheet.translated(tilt * Vec3::new(x, (ROOF_THICKNESS + RIB.y) / 2.0, 0.0))
            };
            front.spawn((Shape::bar(&rib, ROOF_METAL), Finish::Rust));
        }
    }

    /// Fabric panels under the beam, one letter of the name on each, readable
    /// from the street.
    fn spawn_noren(front: &mut ChildSpawnerCommands, beam: &Block) {
        let panels = NAME.len() as f32;
        let width = (beam.size.x - NOREN_GAP * (panels - 1.0)) / panels;
        for (index, &letter) in NAME.iter().enumerate() {
            let left = beam.min().x + index as f32 * (width + NOREN_GAP);
            let panel = Block::between(
                Vec3::new(
                    left,
                    beam.min().y - NOREN_LENGTH,
                    beam.center.z - NOREN_THICKNESS / 2.0,
                ),
                Vec3::new(
                    left + width,
                    beam.min().y,
                    beam.center.z + NOREN_THICKNESS / 2.0,
                ),
            );
            front.spawn((Shape::cuboid(&panel, NOREN), Finish::Fabric));

            // Printed through the fabric, so it shows faintly from inside.
            for (z, color) in [
                (panel.min().z - 0.003, LETTERS),
                (panel.max().z + 0.003, LETTERS_INSIDE),
            ] {
                let middle = Vec3::new(panel.center.x, panel.center.y, z);
                // The street looks along +Z, so its right is -X.
                let scale = LETTER_STEP * Vec2::new(-1.0, 1.0);
                for (a, b) in ALPHABET[letter].centered_strokes() {
                    let stroke = Bar {
                        from: middle + (a * scale).extend(0.0),
                        to: middle + (b * scale).extend(0.0),
                        section: Vec2::new(STROKE, LETTER_DEPTH),
                        up: Vec3::Z,
                    }
                    .extended(STROKE / 2.0);
                    front.spawn((Shape::bar(&stroke, color), Finish::Fabric));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_game_core::clock::{Clock, PLANET_SIZE};
    use ld60_game_core::layout::counter::CounterLayout;

    use super::*;

    #[test]
    fn the_whole_planet_shows_under_the_noren() {
        let eye = CounterLayout::new().seated_view().translation;
        let bottom = BEAM_HEIGHT - NOREN_LENGTH;
        let distance = eye.z - (FRONT + PILLAR.y / 2.0);
        for clock in (0..).map(Clock::from).take_while(|clock| clock.sets() == 0) {
            let planet = clock.planet();
            let yaw = planet.x.atan2(-planet.z);
            let edge = (bottom - eye.y).atan2(distance / yaw.cos());
            assert!(planet.y.asin() + PLANET_SIZE < edge, "{planet} {edge}");
        }
    }
}
