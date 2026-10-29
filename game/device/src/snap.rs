use bevy::{
    picking::{
        PickingSystems,
        backend::{HitData, PointerHits, ray::RayMap},
        mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings},
        pointer::{PointerId, PointerLocation},
    },
    prelude::*,
};
use ld60_game_core::player::focus::{Focus, Focused};

/// How far from a [`Snap`] part the pointer still catches it, in logical
/// pixels.
const REACH: f32 = 24.0;
/// A hit this close past a part, in meters, still counts as on the part.
const SEEN_SLACK: f32 = 0.0002;

pub(super) struct SnapPlugin;

impl Plugin for SnapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, Snap::hits.in_set(PickingSystems::Backend));
    }
}

/// A small part of a held device that the pointer catches from a little
/// way off, so it is easy to hit.
#[derive(Component, Debug, Default)]
pub(super) struct Snap;

impl Snap {
    /// Puts the nearest part in [`REACH`] of each pointer on top of what
    /// the pointer is over, if nothing hides it. A part need not have a
    /// mesh.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn hits(
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        rays: Res<RayMap>,
        cameras: Query<(&Camera, &GlobalTransform)>,
        pointers: Query<(&PointerId, &PointerLocation)>,
        parts: Query<(Entity, &GlobalTransform, &InheritedVisibility), With<Snap>>,
        parents: Query<&ChildOf>,
        pickables: Query<&Pickable>,
        mut ray_cast: MeshRayCast,
        mut hits: MessageWriter<PointerHits>,
    ) {
        let Some(device) = focused.get().filter(|_| *state.get() == Focus::Device) else {
            return;
        };
        let on_device = |part: Entity| parents.iter_ancestors(part).any(|parent| parent == device);
        for ray_id in rays.iter().map(|(ray_id, _)| ray_id) {
            let Ok((camera, eye)) = cameras.get(ray_id.camera) else {
                continue;
            };
            let Some(pointer) = pointers
                .iter()
                .find(|(id, _)| **id == ray_id.pointer)
                .and_then(|(_, location)| location.location())
            else {
                continue;
            };
            let nearest = parts
                .iter()
                .filter(|(part, _, shown)| shown.get() && on_device(*part))
                .filter_map(|(part, at, _)| {
                    let at = at.translation();
                    let screen = camera.world_to_viewport(eye, at).ok()?;
                    let distance = screen.distance(pointer.position);
                    (distance <= REACH).then_some((part, at, distance))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2));
            let Some((part, at, _)) = nearest else {
                continue;
            };
            let Ok(towards) = Dir3::new(at - eye.translation()) else {
                continue;
            };
            let hoverable = |entity| pickables.get(entity).ok().is_none_or(|p| p.is_hoverable);
            let settings = MeshRayCastSettings::default().with_filter(&hoverable);
            let reach = eye.translation().distance(at);
            let first = ray_cast
                .cast_ray(Ray3d::new(eye.translation(), towards), &settings)
                .first()
                .map(|&(entity, ref hit)| (entity, hit.distance));
            // A part is seen if the ray hits it first, or nothing on the
            // way to it, like the empty mouth of a hole.
            let seen = first.is_none_or(|(hit, depth)| {
                hit == part
                    || parents.iter_ancestors(hit).any(|parent| parent == part)
                    || depth >= reach - SEEN_SLACK
            });
            if seen {
                let depth = first.map_or(reach, |(_, depth)| depth.min(reach));
                hits.write(PointerHits::new(
                    ray_id.pointer,
                    vec![(part, HitData::new(ray_id.camera, depth, Some(at), None))],
                    camera.order as f32 + 0.25,
                ));
            }
        }
    }
}
