mod drops;
mod shape;

use std::f32::consts::{PI, TAU};
use std::ops::Range;

use bevy::{light::NotShadowCaster, material::OpaqueRendererMethod, prelude::*};
use rand::{Rng, RngExt};

use super::Pick;
use super::shadows::Blob;

pub(super) use self::drops::DropsPlugin;
/// Ribs, each with a gore of cloth to the next.
const RIBS: usize = shape::RIBS;
/// Radius of the canopy, before size.
const RADIUS: f32 = 0.58;
/// From the hand to the top of the canopy, before size.
pub(super) const SHAFT: f32 = 0.74;
/// Seconds it takes to open or close, and to react to the rain.
const OPENING: f32 = 1.8;
/// Opening goes in steps, each to the share of the way it ends at: it is
/// raised out in front, the other hand pushes the runner up while the
/// canopy unfolds, and then it is lifted overhead. Closing runs them
/// backwards and shakes the water off while it is still in front.
const RAISED: f32 = 0.28;
const UNFOLDED: f32 = 0.78;
/// Out in front it points forward and up.
const IN_FRONT: Vec3 = Vec3::new(0.15, 0.6, 0.8);
/// The canopy snaps a little past open at the end of unfolding.
const POP: f32 = 0.07;
/// Closed, it is shaken this many times, this far, while it is still in
/// front.
const SHAKES: f32 = 3.0;
const SHAKE: f32 = 0.35;
/// The runner the other hand pushes, as a share of the shaft, closed and
/// open.
const RUNNER: Range<f32> = 0.15..0.62;
const REACTION: Range<f32> = 0.3..4.0;
/// Held open, the shaft leans this way from upright, in the holder's space:
/// in towards the head.
const LEAN_IN: Vec3 = Vec3::new(-0.08, 1.0, -0.12);
/// Carried closed, it points down and a little forward, like a cane.
const CARRIED: f32 = PI - 0.3;
/// How far a crowded umbrella tilts away, in radians.
const TILT: f32 = 0.4;
/// The cloth lets some light through, and the rain leaves a film on it.
const TRANSMISSION: f32 = 0.35;
const CLOTH_ROUGHNESS: f32 = 0.6;
const WET_COAT: f32 = 0.8;
const WET_COAT_ROUGHNESS: f32 = 0.15;

const CLOTHS: [Color; 7] = [
    Color::srgb_u8(0xc0, 0x34, 0x2c),
    Color::srgb_u8(0x2a, 0x8a, 0x84),
    Color::srgb_u8(0xd8, 0xa8, 0x30),
    Color::srgb_u8(0x6a, 0x40, 0x9a),
    Color::srgb_u8(0x1c, 0x1c, 0x22),
    Color::srgb_u8(0xe0, 0xd8, 0xc8),
    Color::srgb_u8(0x30, 0x5a, 0xa8),
];
const GRIPS: [Color; 3] = [
    Color::srgb_u8(0x1a, 0x16, 0x14),
    Color::srgb_u8(0x5a, 0x34, 0x1c),
    Color::srgb_u8(0xb8, 0xa8, 0x88),
];
const METAL: Color = Color::srgb_u8(0x30, 0x30, 0x34);

/// The meshes and materials all umbrellas share.
#[derive(Debug)]
pub(super) struct Kit {
    canopy: Handle<Mesh>,
    frame: Handle<Mesh>,
    handle: Handle<Mesh>,
    shaft: Handle<Mesh>,
    folded: Handle<Mesh>,
    cloths: Vec<Handle<StandardMaterial>>,
    grips: Vec<Handle<StandardMaterial>>,
    metal: Handle<StandardMaterial>,
}

/// The cloth and frame at the top of an umbrella, open or folding.
#[derive(Component, Debug, Default)]
pub(super) struct Canopy {
    /// From closed at 0 to open at 1.
    pub(super) open: f32,
    /// How hard it is being shaken, from 0 to 1, throwing off drops.
    pub(super) shaken: f32,
    /// The sound of the rain on it, while the player is near.
    pub(super) patter: Option<Entity>,
}

/// An umbrella someone carries, closed or open over their head.
#[derive(Debug)]
pub(super) struct Umbrella {
    /// At the hand, with the shaft along +Y.
    root: Entity,
    canopy: Entity,
    folded: Entity,
    /// From closed at 0 to open at 1.
    open: f32,
    wants_open: bool,
    /// Seconds until it reacts to a change in the rain.
    reaction: f32,
    /// Tilt away from someone close by, in the holder's space.
    tilt: Vec2,
}

impl Kit {
    pub(super) fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        let cloths = CLOTHS
            .iter()
            .map(|&color| {
                materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: CLOTH_ROUGHNESS,
                    diffuse_transmission: TRANSMISSION,
                    clearcoat: WET_COAT,
                    clearcoat_perceptual_roughness: WET_COAT_ROUGHNESS,
                    double_sided: true,
                    cull_mode: None,
                    // Deferred rendering has no room for transmission.
                    opaque_render_method: OpaqueRendererMethod::Forward,
                    ..default()
                })
            })
            .collect();
        let grips = GRIPS
            .iter()
            .map(|&color| {
                materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.35,
                    ..default()
                })
            })
            .collect();
        Self {
            canopy: meshes.add(shape::canopy()),
            frame: meshes.add(shape::frame()),
            handle: meshes.add(shape::handle()),
            shaft: meshes.add(shape::shaft()),
            folded: meshes.add(shape::folded()),
            cloths,
            grips,
            metal: materials.add(StandardMaterial {
                base_color: METAL,
                metallic: 0.8,
                perceptual_roughness: 0.4,
                ..default()
            }),
        }
    }
}

impl Umbrella {
    /// A closed umbrella for someone of `size`, carried by `parent`.
    pub(super) fn spawn(
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        rng: &mut impl Rng,
        size: f32,
    ) -> Self {
        let cloth = kit.cloths.pick(rng).clone();
        let grip = kit.grips.pick(rng).clone();
        let mut canopy = Entity::PLACEHOLDER;
        let mut folded = Entity::PLACEHOLDER;
        let root = parent
            .spawn((
                Transform::from_scale(Vec3::splat(size)),
                Visibility::default(),
            ))
            .with_children(|umbrella| {
                umbrella.spawn((Mesh3d(kit.handle.clone()), MeshMaterial3d(grip)));
                umbrella.spawn((
                    Mesh3d(kit.shaft.clone()),
                    MeshMaterial3d(kit.metal.clone()),
                    Transform::from_scale(Vec3::new(1.0, SHAFT, 1.0)),
                ));
                folded = umbrella
                    .spawn((Mesh3d(kit.folded.clone()), MeshMaterial3d(cloth.clone())))
                    .id();
                canopy = umbrella
                    .spawn((
                        Name::new("canopy"),
                        Canopy::default(),
                        Transform::from_xyz(0.0, SHAFT, 0.0),
                        Visibility::Hidden,
                    ))
                    .with_children(|canopy| {
                        // The cloth is too thin to cast a shadow of its own
                        // in the shop lamp's light.
                        canopy.spawn((
                            Mesh3d(kit.canopy.clone()),
                            MeshMaterial3d(cloth),
                            NotShadowCaster,
                        ));
                        canopy
                            .spawn((Mesh3d(kit.frame.clone()), MeshMaterial3d(kit.metal.clone())));
                    })
                    .id();
            })
            .id();
        Self {
            root,
            canopy,
            folded,
            open: 0.0,
            wants_open: false,
            reaction: rng.random_range(REACTION),
            tilt: Vec2::ZERO,
        }
    }

    /// Holds it `open` of the way through opening, or closing if not
    /// `opening`.
    #[cfg(feature = "debug_port")]
    pub(super) fn hold_open(&mut self, open: f32, opening: bool) {
        self.open = open.clamp(0.0, 1.0);
        self.wants_open = opening;
        self.reaction = f32::INFINITY;
    }

    pub(super) fn is_open(&self) -> bool {
        self.openness() > 0.5
    }

    /// How far the canopy is unfolded, from 0 to 1.
    pub(super) fn openness(&self) -> f32 {
        ease((self.open - RAISED) / (UNFOLDED - RAISED))
    }

    /// How far it is raised out in front, and lifted overhead from there,
    /// each from 0 to 1.
    fn raised(&self) -> (f32, f32) {
        (
            ease(self.open / RAISED),
            ease((self.open - UNFOLDED) / (1.0 - UNFOLDED)),
        )
    }

    /// How hard it is shaken now, from -1 to 1: a few times while closing,
    /// once folded and still in front.
    fn shake(&self) -> f32 {
        if self.wants_open || self.open >= RAISED {
            return 0.0;
        }
        let t = self.open / RAISED;
        (TAU * SHAKES * t).sin() * (PI * t).sin()
    }

    /// How far past open the canopy snaps as it finishes opening.
    fn pop(&self) -> f32 {
        if !self.wants_open {
            return 0.0;
        }
        let t = ((self.open - (UNFOLDED - 0.1)) / 0.18).clamp(0.0, 1.0);
        POP * (PI * t).sin()
    }

    /// Where the other hand goes to push the runner, in the holder's
    /// space, with the umbrella at `placed` for a holder of `size`, and
    /// how firmly from 0 to 1. None while the canopy is not moving.
    pub(super) fn runner(&self, placed: &Transform, size: f32) -> Option<(Vec3, f32)> {
        let unfold = self.openness();
        let grip = ease(unfold / 0.15) * ease((1.0 - unfold) / 0.15);
        let moving = self.open > RAISED * 0.7 && self.open < UNFOLDED + 0.05;
        (moving && grip > 0.0).then(|| {
            let along = RUNNER.start + (RUNNER.end - RUNNER.start) * unfold;
            (placed.transform_point(Vec3::Y * along * SHAFT * size), grip)
        })
    }

    /// Opens it a while after the rain starts, and closes it a while after
    /// it stops.
    pub(super) fn follow_rain(&mut self, raining: bool, dt: f32, rng: &mut impl Rng) {
        if raining != self.wants_open {
            self.reaction -= dt;
            if self.reaction <= 0.0 {
                self.wants_open = raining;
                self.reaction = rng.random_range(REACTION);
            }
        }
        let target = if self.wants_open { 1.0 } else { 0.0 };
        let step = dt / OPENING;
        self.open += (target - self.open).clamp(-step, step);
    }

    /// Tilts it away from `away`, a direction in the holder's space, as far
    /// as `crowded` from 0 to 1.
    pub(super) fn lean_away(&mut self, away: Vec2, crowded: f32, dt: f32) {
        let target = away.normalize_or_zero() * TILT * crowded.clamp(0.0, 1.0);
        self.tilt += (target - self.tilt) * (1.0 - (-4.0 * dt).exp());
    }

    /// Where the hand holds it: at `carry` while closed, out at `front`
    /// while opening or closing, and at `hold` while open, all in the same
    /// space.
    pub(super) fn hand(&self, carry: Vec3, front: Vec3, hold: Vec3) -> Vec3 {
        let (raised, lifted) = self.raised();
        let shaken = Vec3::Y * 0.02 * self.shake();
        carry.lerp(front, raised).lerp(hold, lifted) + shaken
    }

    /// The umbrella in the holder's space with its handle at `hand`, which
    /// swings by `swing` radians while carried closed.
    pub(super) fn place(&self, hand: Vec3, swing: f32) -> Transform {
        let (raised, lifted) = self.raised();
        let carried = Quat::from_rotation_x(CARRIED + swing);
        let front = Quat::from_rotation_arc(Vec3::Y, IN_FRONT.normalize());
        let held = LEAN_IN + Vec3::new(self.tilt.x, 0.0, self.tilt.y);
        let held = Quat::from_rotation_arc(Vec3::Y, held.normalize());
        let shaken = Quat::from_rotation_x(SHAKE * self.shake());
        let turn = carried.slerp(front, raised).slerp(held, lifted) * shaken;
        Transform::from_translation(hand).with_rotation(turn)
    }

    /// How hard it is being shaken, from 0 to 1.
    pub(super) fn shaken(&self) -> f32 {
        self.shake().abs()
    }

    /// Moves the parts for `placed` and a holder of `size`.
    pub(super) fn pose(
        &self,
        placed: Transform,
        size: f32,
        set: &mut impl FnMut(Entity, Transform),
        show: &mut impl FnMut(Entity, bool),
    ) {
        set(self.root, placed.with_scale(Vec3::splat(size)));
        let open = self.openness();
        let spread = 0.12 + 0.88 * open + self.pop();
        let height = 1.0 + 2.5 * (1.0 - open);
        set(
            self.canopy,
            Transform::from_xyz(0.0, SHAFT, 0.0)
                .with_scale(Vec3::new(spread, height, spread) * RADIUS),
        );
        show(self.canopy, self.open > 0.02);
        show(self.folded, self.open < 0.2);
    }

    /// The open canopy as a shadow shape, for `placed` and a holder of
    /// `size`.
    pub(super) fn shadow(&self, placed: &Transform, size: f32) -> Option<Blob> {
        (self.openness() > 0.3).then(|| {
            let rise = shape::rise() * RADIUS;
            let middle = placed.transform_point(Vec3::Y * (SHAFT - rise / 2.0) * size);
            let spread = (0.12 + 0.88 * self.openness() + self.pop()) * RADIUS * 2.0;
            Blob::new(
                &Transform::from_translation(middle).with_rotation(placed.rotation),
                Vec3::new(spread, rise, spread) * size,
            )
        })
    }

    pub(super) fn canopy(&self) -> Entity {
        self.canopy
    }
}

/// `t` from 0 to 1, eased in and out.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
