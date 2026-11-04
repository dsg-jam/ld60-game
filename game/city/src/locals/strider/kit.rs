use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use rand::Rng;

use super::super::plumage::Feathers;
use super::{ARM, HEAD, NECK, NECK_BASE, THIGH};

/// Different coats, crests and tails to pick from.
const VARIANTS: usize = 4;
/// How far the feathers cover the top of the legs and the arms.
const THIGH_CUFF: f32 = 0.24;
const ARM_CUFF: f32 = 0.16;
/// Three toes forward and one back, with their spread and length.
const TOES: [(f32, f32); 4] = [(-0.45, 0.13), (0.0, 0.16), (0.45, 0.13), (PI, 0.07)];
const TOE: f32 = 0.014;

/// The meshes all striders share, at size 1.
#[derive(Debug)]
pub(in super::super) struct Kit {
    pub(super) bodies: Vec<Handle<Mesh>>,
    pub(super) neck: Handle<Mesh>,
    pub(super) crests: Vec<Handle<Mesh>>,
    pub(super) tails: Vec<Handle<Mesh>>,
    pub(super) cuff: Handle<Mesh>,
    pub(super) sleeve: Handle<Mesh>,
    pub(super) toes: Handle<Mesh>,
}

impl Kit {
    pub(in super::super) fn new(meshes: &mut Assets<Mesh>, rng: &mut impl Rng) -> Self {
        let bodies = (0..VARIANTS)
            .map(|_| meshes.add(Feathers::body(rng)))
            .collect();
        let crests = (0..VARIANTS)
            .map(|_| meshes.add(Feathers::crest(rng)))
            .collect();
        let tails = (0..VARIANTS)
            .map(|_| meshes.add(Feathers::tail(rng)))
            .collect();
        Self {
            bodies,
            neck: meshes.add(Feathers::neck(NECK_BASE, HEAD, NECK / 2.0, rng)),
            crests,
            tails,
            cuff: meshes.add(Feathers::cuff(THIGH_CUFF, THIGH / 2.0, rng)),
            sleeve: meshes.add(Feathers::cuff(ARM_CUFF, ARM / 2.0, rng)),
            toes: meshes.add(Self::toes()),
        }
    }

    /// Thin toes from the ankle at the origin, the front towards +Z.
    fn toes() -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new())
        .with_inserted_indices(Indices::U32(Vec::new()));
        for (angle, length) in TOES {
            let along = Quat::from_rotation_y(angle) * Vec3::Z;
            let tip = along * length + Vec3::Y * TOE;
            let toe = Capsule3d::new(TOE, length)
                .mesh()
                .longitudes(6)
                .latitudes(4)
                .build()
                .rotated_by(Quat::from_rotation_arc(Vec3::Y, tip.normalize()))
                .translated_by(tip / 2.0 + Vec3::Y * TOE);
            let mut toe = toe;
            toe.remove_attribute(Mesh::ATTRIBUTE_UV_0);
            if let Err(err) = mesh.merge(&toe) {
                error!(error = &err as &dyn std::error::Error, "toe");
            }
        }
        mesh
    }
}
