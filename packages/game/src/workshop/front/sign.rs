use bevy::prelude::*;
use ld60_devices::DeviceKind;
use rand::rngs::SmallRng;
use rand::{Rng, RngExt, SeedableRng};

use super::{BEAM_HEIGHT, Front};
use crate::device::Silhouette;
use crate::geometry::bar::Bar;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;
use crate::parts::script::{GLYPH_SIZE, Word};
use crate::workshop::run::Run;
use crate::workshop::shop::{FRONT, PILLAR};

const BOARD: Vec3 = Vec3::new(0.3, 0.22, 0.012);
/// Height of the top edge of the signs.
const TOP: f32 = 2.47;
/// How far in front of the middle of the beam the signs hang, towards the
/// street. Just in front of the noren, with the cords still under the beam.
const OUT: f32 = 0.022;
/// Where the signs hang along the beam, in the order the shop got them.
const SLOTS: [f32; 6] = [-0.85, 0.85, -0.5, 0.5, -0.15, 0.15];
/// The cords hang this far from the middle of the board.
const CORD_SPREAD: f32 = 0.1;
const CORD: f32 = 0.005;
const PAINT: f32 = 0.0008;
/// The part of the board the device is painted on.
const PICTURE: Vec2 = Vec2::new(0.24, 0.13);
/// Distance of the middle of the picture below the top edge.
const PICTURE_DROP: f32 = 0.09;
const LETTER_HEIGHT: f32 = 0.032;
/// Distance of the middle of the lettering below the top edge.
const LETTERING_DROP: f32 = 0.185;
const STROKE: f32 = 0.006;
/// The painter gets the proportions wrong by up to this share.
const STRETCH: f32 = 0.2;
/// In radians.
const TILT: f32 = 0.15;
/// How far a part may slip, as a share of its size.
const SLIP: f32 = 0.25;
/// How far a stroke end may slip, in glyph grid steps.
const SHAKE: f32 = 0.15;
/// The painter picks colors up to this far around the color wheel, in
/// degrees.
const HUE_SHIFT: f32 = 70.0;
const BOARD_COLOR: Color = Color::srgb_u8(0xd8, 0xc8, 0xa0);
const CORD_COLOR: Color = Color::srgb_u8(0x18, 0x16, 0x14);

pub(super) struct SignPlugin;

impl Plugin for SignPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(WindowSign::on_add).add_systems(
            Update,
            WindowSign::hang.run_if(resource_exists_and_changed::<Run>),
        );
    }
}

/// A sign in the shop window that tells the locals which device the shop
/// repairs.
///
/// A local sign painter copied it from a dead unit. The painter does not
/// know human devices, so the proportions, angles and colors are off in a
/// way that `seed` decides. The local name of the device is written below.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(Transform, Visibility)]
pub(crate) struct WindowSign {
    device: DeviceKind,
    seed: u64,
}

/// A patch of paint on a sign, in meters from the middle of the top edge.
/// +X is to the right and +Y up, as seen from the street.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Dab {
    center: Vec2,
    size: Vec2,
    /// Counterclockwise as seen from the street, in radians.
    angle: f32,
    color: Color,
    /// Dabs of a higher layer lie on those of a lower one.
    layer: u8,
}

impl WindowSign {
    /// The sign of `device` in run `run`. Each run gets other paintings.
    fn new(device: DeviceKind, run: u32) -> Self {
        let index = DeviceKind::ALL
            .iter()
            .position(|&kind| kind == device)
            .unwrap_or_default();
        Self {
            device,
            seed: (u64::from(run) << 8) | index as u64,
        }
    }

    /// Hangs a sign for each device the shop repairs, and takes down the
    /// others.
    fn hang(
        run: Res<Run>,
        signs: Query<(Entity, &WindowSign)>,
        fronts: Query<Entity, With<Front>>,
        mut commands: Commands,
    ) {
        let wanted: Vec<_> = run
            .signs()
            .iter()
            .map(|&device| Self::new(device, run.number()))
            .collect();
        let hung = signs
            .iter()
            .filter(|(_, sign)| wanted.contains(sign))
            .count();
        if hung == wanted.len() && signs.iter().count() == wanted.len() {
            return;
        }
        for (entity, _) in &signs {
            commands.entity(entity).despawn();
        }
        let z = FRONT + PILLAR.y / 2.0 - OUT;
        for front in &fronts {
            commands.entity(front).with_children(|front| {
                for (sign, x) in wanted.iter().zip(SLOTS) {
                    front.spawn((
                        Name::new("window sign"),
                        *sign,
                        Transform::from_xyz(x, TOP, z),
                    ));
                }
            });
        }
    }

    fn on_add(add: On<Add<WindowSign>>, signs: Query<&WindowSign>, mut commands: Commands) {
        let Ok(sign) = signs.get(add.entity) else {
            return;
        };
        let mut rng = SmallRng::seed_from_u64(sign.seed);
        let board = Block::between(
            Vec3::new(-BOARD.x / 2.0, -BOARD.y, -BOARD.z / 2.0),
            Vec3::new(BOARD.x / 2.0, 0.0, BOARD.z / 2.0),
        );
        let dabs = Self::picture(&Silhouette::of(sign.device), &mut rng);
        let ink: Color = Hsla::new(rng.random_range(0.0..360.0), 0.6, 0.18, 1.0).into();
        let strokes = Self::lettering(&mut rng);
        commands.entity(add.entity).with_children(|sign| {
            sign.spawn((Shape::cuboid(&board, BOARD_COLOR), Finish::Wood));
            for side in [-1.0, 1.0] {
                let cord = Block::between(
                    Vec3::new(side * CORD_SPREAD - CORD / 2.0, 0.0, -CORD / 2.0),
                    Vec3::new(
                        side * CORD_SPREAD + CORD / 2.0,
                        BEAM_HEIGHT - TOP,
                        CORD / 2.0,
                    ),
                );
                sign.spawn((Shape::cuboid(&cord, CORD_COLOR), Finish::Fabric));
            }
            // Painted on both faces, like on glass, so the inside shows the
            // picture mirrored.
            for face in [-1.0, 1.0] {
                for dab in &dabs {
                    let z = face * (BOARD.z / 2.0 + PAINT * (f32::from(dab.layer) + 0.5));
                    sign.spawn((
                        Shape::cuboid(&Block::new(dab.size.extend(PAINT)), dab.color),
                        Finish::Paint,
                    ))
                    .insert(
                        // The street looks along +Z, so its right is -X.
                        Transform::from_xyz(-dab.center.x, dab.center.y, z)
                            .with_rotation(Quat::from_rotation_z(-dab.angle)),
                    );
                }
                let z = face * (BOARD.z / 2.0 + PAINT * 1.5);
                for &(a, b) in &strokes {
                    let stroke = Bar {
                        from: Vec3::new(-a.x, a.y, z),
                        to: Vec3::new(-b.x, b.y, z),
                        section: Vec2::new(STROKE, PAINT),
                        up: Vec3::Z,
                    }
                    .extended(STROKE / 2.0);
                    sign.spawn((Shape::bar(&stroke, ink), Finish::Paint));
                }
            }
        });
    }

    /// The device as the painter copies it.
    fn picture(silhouette: &Silhouette, rng: &mut impl Rng) -> Vec<Dab> {
        let bounds = silhouette.bounds();
        let stretch = Vec2::new(
            rng.random_range(1.0 - STRETCH..1.0 + STRETCH),
            rng.random_range(1.0 - STRETCH..1.0 + STRETCH),
        );
        let scale = (PICTURE / (bounds.size() * stretch)).min_element();
        let tilt = rng.random_range(-TILT..TILT);
        let hue = rng.random_range(-HUE_SHIFT..HUE_SHIFT);
        let middle = Vec2::new(0.0, -PICTURE_DROP);
        silhouette
            .parts()
            .iter()
            .enumerate()
            .map(|(index, &(part, color))| {
                let slip = if index == 0 {
                    Vec2::ZERO
                } else {
                    Vec2::new(rng.random_range(-SLIP..SLIP), rng.random_range(-SLIP..SLIP))
                        * part.size()
                };
                let offset = (part.center() - bounds.center() + slip) * stretch * scale;
                let color = Hsla::from(Self::alien(color, rng));
                Dab {
                    center: middle + Vec2::from_angle(tilt).rotate(offset),
                    size: part.size() * stretch * scale * rng.random_range(0.85..1.15),
                    angle: tilt + rng.random_range(-TILT..TILT) * f32::from(index > 0),
                    color: color.rotate_hue(hue).into(),
                    layer: u8::from(index > 0),
                }
            })
            .collect()
    }

    /// The local name of the device, in shaky strokes.
    fn lettering(rng: &mut impl Rng) -> Vec<(Vec2, Vec2)> {
        let word = Word::random(rng, 3..6);
        let fit = PICTURE.x / word.size(false).x;
        let step = (LETTER_HEIGHT / GLYPH_SIZE.y).min(fit);
        let middle = Vec2::new(0.0, -LETTERING_DROP);
        let mut shake = || {
            Vec2::new(
                rng.random_range(-SHAKE..SHAKE),
                rng.random_range(-SHAKE..SHAKE),
            )
        };
        word.strokes(false)
            .into_iter()
            .map(|(a, b)| {
                let a = middle + (a + shake()) * step;
                let b = middle + (b + shake()) * step;
                (a, b)
            })
            .collect()
    }

    /// A brighter, stronger color than the human one.
    fn alien(color: Color, rng: &mut impl Rng) -> Color {
        let hsla = Hsla::from(color);
        Hsla {
            hue: (hsla.hue + rng.random_range(-15.0..15.0)).rem_euclid(360.0),
            saturation: (hsla.saturation * 1.5 + 0.2).min(1.0),
            lightness: hsla.lightness.clamp(0.15, 0.8),
            alpha: 1.0,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workshop::salvage::Find;

    fn dabs(device: DeviceKind, seed: u64) -> Vec<Dab> {
        WindowSign::picture(&Silhouette::of(device), &mut SmallRng::seed_from_u64(seed))
    }

    #[test]
    fn each_seed_paints_another_picture() {
        assert_eq!(dabs(DeviceKind::Summa6, 1), dabs(DeviceKind::Summa6, 1));
        assert_ne!(dabs(DeviceKind::Summa6, 1), dabs(DeviceKind::Summa6, 2));
    }

    #[test]
    fn the_paint_stays_on_the_board() {
        for device in DeviceKind::ALL {
            for seed in 0..50 {
                for dab in dabs(device, seed) {
                    let reach = dab.center.abs() + dab.size.length() / 2.0;
                    assert!(reach.x < BOARD.x / 2.0, "{device:?} {seed} {dab:?}");
                    assert!(dab.center.y < 0.0 && dab.center.y > -BOARD.y);
                }
                let strokes = WindowSign::lettering(&mut SmallRng::seed_from_u64(seed));
                for (a, b) in strokes {
                    for point in [a, b] {
                        assert!(point.x.abs() < BOARD.x / 2.0, "{seed} {point}");
                        assert!(point.y < 0.0 && point.y > -BOARD.y, "{seed} {point}");
                    }
                }
            }
        }
    }

    #[test]
    fn buying_a_device_hangs_its_sign() {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.spawn(Front);
        let signs = |world: &mut World| {
            world
                .query::<&WindowSign>()
                .iter(world)
                .map(|sign| sign.device)
                .collect::<Vec<_>>()
        };
        world.run_system_cached(WindowSign::hang).unwrap();
        assert_eq!(signs(&mut world), [DeviceKind::Tally2]);
        let mut run = world.resource_mut::<Run>();
        run.add_till(100);
        assert!(run.buy(Find::Device(DeviceKind::Summa6)));
        world.run_system_cached(WindowSign::hang).unwrap();
        let mut hung = signs(&mut world);
        hung.sort_by_key(|device| format!("{device:?}"));
        assert_eq!(hung, [DeviceKind::Summa6, DeviceKind::Tally2]);
        let next = world.resource::<Run>().next();
        world.insert_resource(next);
        world.run_system_cached(WindowSign::hang).unwrap();
        assert_eq!(signs(&mut world), [DeviceKind::Tally2]);
    }
}
