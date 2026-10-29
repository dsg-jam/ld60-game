use std::iter;

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    camera::primitives::Aabb,
    light::NotShadowCaster,
    math::Affine3A,
    picking::hover::HoverMap,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

use super::{
    camera::PlayerCamera,
    focus::{self, Clickable, Disabled, Focus},
};

const COLOR: LinearRgba = LinearRgba::new(1.0, 0.98, 0.9, 1.0);
/// In depth-texture units, so the overlay wins against its own mesh.
const DEPTH_BIAS: f32 = 16.0;
/// Seconds to fade in or out.
const FADE_TIME: f32 = 0.25;
/// Seconds for the band to cross the object once.
const SWEEP_TIME: f32 = 0.9;
/// Seconds from the start of one sweep to the next.
const SWEEP_PERIOD: f32 = 2.0;
/// In screen units, where 1 is 45 degrees away from the view center.
const BAND_WIDTH: f32 = 0.05;
/// The band moves along this screen direction, with y pointing up.
const SWEEP_DIRECTION: Vec2 = Vec2::new(0.8, -0.6);

pub(super) struct ShimmerPlugin;

impl Plugin for ShimmerPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shimmer.wesl");
        // After `Update` has applied its despawns, so a hovered target that
        // was just removed is gone instead of getting a shimmer.
        app.add_plugins(MaterialPlugin::<ShimmerMaterial>::default())
            .add_systems(
                PostUpdate,
                (Shimmer::start, Shimmer::follow, Shimmer::update)
                    .chain()
                    .before(TransformSystems::Propagate),
            );
    }
}

/// Light that sweeps over a hovered [`Clickable`].
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct ShimmerMaterial {
    #[uniform(0)]
    params: ShimmerParams,
}

/// Matches `Shimmer` in `shimmer.wesl`.
#[derive(ShaderType, Debug, Clone)]
struct ShimmerParams {
    color: LinearRgba,
    direction: Vec2,
    /// From 0 (invisible) to 1 (fully faded in).
    level: f32,
    /// Center of the band along `direction`, in screen units.
    band: f32,
    band_width: f32,
}

impl Material for ShimmerMaterial {
    fn fragment_shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("shimmer.wesl"))
            .with_source("embedded")
            .into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn depth_bias(&self) -> f32 {
        DEPTH_BIAS
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

/// Copy of a mesh that draws the shimmer on top of it.
#[derive(Component)]
struct ShimmerOverlay;

/// Shimmer on a [`Clickable`] while it fades in or out.
#[derive(Component)]
struct Shimmer {
    level: f32,
    /// Seconds since the hover started.
    elapsed: f32,
    hovered: bool,
    material: Handle<ShimmerMaterial>,
}

impl Shimmer {
    /// The target under the mouse in the desk view, if any. A disabled one
    /// does not shimmer.
    fn hovered(
        state: &State<Focus>,
        hovers: &HoverMap,
        targets: &Query<Has<Disabled>, With<Clickable>>,
        parents: &Query<&ChildOf>,
    ) -> Option<Entity> {
        if *state.get() != Focus::Desk {
            return None;
        }
        focus::under_mouse(hovers, parents)
            .find_map(|entity| targets.get(entity).ok().map(|disabled| (entity, disabled)))
            .filter(|&(_, disabled)| !disabled)
            .map(|(entity, _)| entity)
    }

    /// Starts a shimmer on a newly hovered target.
    fn start(
        state: Res<State<Focus>>,
        hovers: Res<HoverMap>,
        targets: Query<Has<Disabled>, With<Clickable>>,
        parents: Query<&ChildOf>,
        shimmers: Query<(), With<Shimmer>>,
        mut materials: ResMut<Assets<ShimmerMaterial>>,
        mut commands: Commands,
    ) {
        let Some(target) = Self::hovered(&state, &hovers, &targets, &parents) else {
            return;
        };
        if shimmers.contains(target) {
            return;
        }
        let material = materials.add(ShimmerMaterial {
            params: ShimmerParams {
                color: COLOR,
                direction: SWEEP_DIRECTION.normalize(),
                level: 0.0,
                band: 0.0,
                band_width: BAND_WIDTH,
            },
        });
        commands.entity(target).insert(Shimmer {
            level: 0.0,
            elapsed: 0.0,
            hovered: true,
            material,
        });
    }

    /// Keeps an overlay on each mesh of a shimmering target, with the mesh
    /// the target shows now. Targets that animate swap their meshes or add
    /// new ones while they shimmer.
    fn follow(
        shimmers: Query<(Entity, &Shimmer)>,
        children: Query<&Children>,
        meshes: Query<&Mesh3d, Without<ShimmerOverlay>>,
        mut overlays: Query<&mut Mesh3d, With<ShimmerOverlay>>,
        mut commands: Commands,
    ) {
        for (target, shimmer) in &shimmers {
            for entity in iter::once(target).chain(children.iter_descendants(target)) {
                let Ok(mesh) = meshes.get(entity) else {
                    continue;
                };
                let overlay = children
                    .get(entity)
                    .ok()
                    .and_then(|kids| kids.iter().find(|&kid| overlays.contains(kid)));
                match overlay.and_then(|overlay| overlays.get_mut(overlay).ok()) {
                    Some(mut overlay) => {
                        if overlay.0 != mesh.0 {
                            overlay.0 = mesh.0.clone();
                        }
                    }
                    None => {
                        commands.entity(entity).with_child((
                            ShimmerOverlay,
                            mesh.clone(),
                            MeshMaterial3d(shimmer.material.clone()),
                            NotShadowCaster,
                            Pickable::IGNORE,
                        ));
                    }
                }
            }
        }
    }

    /// Extent of the visible overlays on `target` along [`SWEEP_DIRECTION`]
    /// on screen.
    ///
    /// Screen units are view space x and y divided by the depth, so a band
    /// at a fixed value is a straight line on screen.
    fn extent(
        target: Entity,
        view_from_world: Affine3A,
        children: &Query<&Children>,
        overlays: &Query<(&Aabb, &GlobalTransform, &InheritedVisibility), With<ShimmerOverlay>>,
    ) -> Option<(f32, f32)> {
        let direction = SWEEP_DIRECTION.normalize();
        let mut extent: Option<(f32, f32)> = None;
        for (aabb, transform, _) in children
            .iter_descendants(target)
            .filter_map(|entity| overlays.get(entity).ok())
            .filter(|(_, _, visibility)| visibility.get())
        {
            let center = Vec3::from(aabb.center);
            let half = Vec3::from(aabb.half_extents);
            let view_from_local = view_from_world * transform.affine();
            for corner in 0..8 {
                let sign = Vec3::new(
                    if corner & 1 == 0 { -1.0 } else { 1.0 },
                    if corner & 2 == 0 { -1.0 } else { 1.0 },
                    if corner & 4 == 0 { -1.0 } else { 1.0 },
                );
                let point = view_from_local.transform_point3(center + sign * half);
                // Corners behind the camera have no screen position.
                if point.z >= 0.0 {
                    continue;
                }
                let along = direction.dot(point.truncate() / -point.z);
                extent = Some(match extent {
                    Some((from, to)) => (from.min(along), to.max(along)),
                    None => (along, along),
                });
            }
        }
        extent
    }

    /// Fades shimmers in or out, moves the band, and removes faded out ones.
    ///
    /// Each hover starts with the band outside the object.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn update(
        time: Res<Time>,
        state: Res<State<Focus>>,
        hovers: Res<HoverMap>,
        targets: Query<Has<Disabled>, With<Clickable>>,
        parents: Query<&ChildOf>,
        children: Query<&Children>,
        overlays: Query<(&Aabb, &GlobalTransform, &InheritedVisibility), With<ShimmerOverlay>>,
        camera: Single<&GlobalTransform, With<PlayerCamera>>,
        mut shimmers: Query<(Entity, &mut Shimmer)>,
        mut materials: ResMut<Assets<ShimmerMaterial>>,
        mut commands: Commands,
    ) {
        let hovered = Self::hovered(&state, &hovers, &targets, &parents);
        let dt = time.delta_secs();
        let view_from_world = camera.affine().inverse();
        for (target, mut shimmer) in &mut shimmers {
            let now_hovered = hovered == Some(target);
            if now_hovered && !shimmer.hovered {
                shimmer.elapsed = 0.0;
            }
            shimmer.hovered = now_hovered;
            shimmer.elapsed += dt;
            let step = if now_hovered { dt } else { -dt } / FADE_TIME;
            shimmer.level = (shimmer.level + step).clamp(0.0, 1.0);

            let progress = (shimmer.elapsed % SWEEP_PERIOD / SWEEP_TIME).min(1.0);
            let extent = Self::extent(target, view_from_world, &children, &overlays);
            if let (Some((from, to)), Some(mut material)) =
                (extent, materials.get_mut(&shimmer.material))
            {
                let start = from - 2.0 * BAND_WIDTH;
                let end = to + 2.0 * BAND_WIDTH;
                material.params.level = shimmer.level;
                material.params.band = start + (end - start) * progress;
            }

            if shimmer.level == 0.0 {
                for entity in children.iter_descendants(target) {
                    if overlays.contains(entity) {
                        commands.entity(entity).despawn();
                    }
                }
                commands.entity(target).remove::<Shimmer>();
            }
        }
    }
}
