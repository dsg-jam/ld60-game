use std::f32::consts::PI;

use bevy::prelude::*;
use ld60_geometry::triangles::Triangles;

use super::skeleton::Side;

/// The sides of the dishes: the cup is narrow and tall, the bowl wide.
const CUP: [Vec2; 5] = [
    Vec2::new(0.0, 0.0),
    Vec2::new(0.018, 0.0),
    Vec2::new(0.02, 0.004),
    Vec2::new(0.024, 0.05),
    Vec2::new(0.022, 0.05),
];
const BOWL: [Vec2; 6] = [
    Vec2::new(0.0, 0.0),
    Vec2::new(0.022, 0.0),
    Vec2::new(0.024, 0.006),
    Vec2::new(0.048, 0.026),
    Vec2::new(0.052, 0.034),
    Vec2::new(0.048, 0.034),
];
/// How full each is.
const CUP_FULL: f32 = 0.04;
const BOWL_FULL: f32 = 0.024;

/// What a bird eats or drinks at a stall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum Food {
    /// Tea in a cup, held in the right hand.
    Cup,
    /// Soup in a bowl, held in the left hand.
    Bowl,
}

/// What a bird does with its food now, set by its outing.
#[derive(Debug, Default, Clone, Copy)]
pub(in super::super) struct Meal {
    pub(in super::super) food: Option<Food>,
    /// Whether it holds the food, rather than it standing on the counter.
    pub(in super::super) held: bool,
    /// How far it reaches over the counter, from 0 to 1.
    pub(in super::super) reach: f32,
    /// How far through a bite or a sip, from 0 to 1.
    pub(in super::super) bite: f32,
}

impl Food {
    pub(in super::super) const ALL: [Self; 2] = [Self::Cup, Self::Bowl];

    /// Its dish, open at the top.
    pub(in super::super) fn dish(self) -> Mesh {
        let mut mesh = Triangles::default();
        let profile: &[Vec2] = match self {
            Self::Cup => &CUP,
            Self::Bowl => &BOWL,
        };
        mesh.lathe(profile, 16);
        mesh.build()
    }

    /// What is in it: a disc at the height it is filled to.
    pub(in super::super) fn filling(self) -> Mesh {
        Circle::new(self.surface().x)
            .mesh()
            .resolution(16)
            .build()
            .rotated_by(Quat::from_rotation_x(-PI / 2.0))
            .translated_by(Vec3::Y * self.surface().y)
    }

    /// How wide and how high up the top of what is in it is.
    pub(in super::super) fn surface(self) -> Vec2 {
        match self {
            Self::Cup => Vec2::new(0.0225, CUP_FULL),
            Self::Bowl => Vec2::new(0.044, BOWL_FULL),
        }
    }

    /// The hand that holds it.
    pub(super) fn side(self) -> Side {
        match self {
            Self::Cup => Side::Right,
            Self::Bowl => Side::Left,
        }
    }

    pub(in super::super) fn index(self) -> usize {
        match self {
            Self::Cup => 0,
            Self::Bowl => 1,
        }
    }
}

impl Meal {
    /// How far the head dips into the food and how far it tips back
    /// after, both from 0 to 1, a bite or sip in: down, a moment there,
    /// up and back to swallow, and level again.
    pub(in super::super) fn dip(self) -> (f32, f32) {
        let t = self.bite;
        let ease = |t: f32| {
            let t = t.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let down = ease(t / 0.35) * (1.0 - ease((t - 0.5) / 0.2));
        let back = (PI * ((t - 0.55) / 0.45).clamp(0.0, 1.0)).sin();
        (down, back)
    }
}
