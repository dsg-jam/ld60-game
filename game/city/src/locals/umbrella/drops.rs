use std::f32::consts::TAU;

use bevy::{
    asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use ld60_game_core::GRAVITY;
use ld60_game_player::camera::PlayerCamera;
use ld60_geometry::quads::Quads;
use rand::RngExt;

use super::super::super::weather::Weather;
use super::super::Whims;
use super::super::shadows::Lamps;
use super::{Canopy, RIBS, shape};

/// Drops falling at once, at most.
const SLOTS: usize = 512;
/// Drops a second off each rib tip in full rain.
const PER_TIP: f32 = 4.0;
/// Only umbrellas this near the player drip.
const NEAR: f32 = 16.0;
const WIDTH: f32 = 0.0015;
/// Lux that makes a drop as bright as its color.
const FULL_LIGHT: f32 = 300.0;
const BRIGHTEST: f32 = 3.0;

pub(in super::super) struct DropsPlugin;

impl Plugin for DropsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "drops.wesl");
        app.add_plugins(MaterialPlugin::<DropMaterial>::default())
            .add_systems(Startup, Drops::spawn)
            .add_systems(PostUpdate, Drops::shed.after(TransformSystems::Propagate));
    }
}

/// Drops running off the rib tips of open umbrellas. Each slot holds a drop
/// that the vertex shader lets fall from where and when it left its tip.
#[derive(Resource, Debug)]
struct Drops {
    mesh: Handle<Mesh>,
    anchors: Vec<[f32; 3]>,
    births: Vec<[f32; 3]>,
    lights: Vec<[f32; 4]>,
    next: usize,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct DropMaterial {
    #[uniform(0)]
    params: DropParams,
}

/// Matches `Drops` in `drops.wesl`.
#[derive(ShaderType, Debug, Clone)]
struct DropParams {
    width: f32,
    gravity: f32,
}

impl Drops {
    fn spawn(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<DropMaterial>>,
    ) {
        let mut quads = Quads::default();
        for _ in 0..SLOTS {
            quads.push_with(Vec3::ZERO, [-1000.0, 0.0, 0.0], [0.0; 4]);
        }
        let mut mesh = quads.into_mesh();
        // Kept on the CPU too, as new drops change it every frame.
        mesh.asset_usage = RenderAssetUsages::default();
        let mesh = meshes.add(mesh);
        commands.spawn((
            Name::new("drops off umbrellas"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(DropMaterial {
                params: DropParams {
                    width: WIDTH,
                    gravity: GRAVITY,
                },
            })),
            Transform::default(),
            Quads::unbounded(),
        ));
        commands.insert_resource(Self {
            mesh,
            anchors: vec![[0.0; 3]; SLOTS * 4],
            births: vec![[-1000.0, 0.0, 0.0]; SLOTS * 4],
            lights: vec![[0.0; 4]; SLOTS * 4],
            next: 0,
        });
    }

    /// Lets drops go from the rib tips of the open umbrellas near the
    /// player, the more the harder it rains.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn shed(
        time: Res<Time>,
        weather: Res<Weather>,
        lamps: Res<Lamps>,
        camera: Single<&GlobalTransform, With<PlayerCamera>>,
        canopies: Query<(&Canopy, &GlobalTransform)>,
        mut whims: ResMut<Whims>,
        mut drops: ResMut<Self>,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        let eye = camera.translation();
        let now = time.elapsed_secs_wrapped();
        let chance = (PER_TIP * weather.rain * time.delta_secs()).min(1.0);
        let rng = &mut whims.0;
        let mut shed = false;
        for (canopy, at) in &canopies {
            if canopy.open < 0.9 || at.translation().distance(eye) > NEAR {
                continue;
            }
            for rib in 0..RIBS {
                if rng.random::<f32>() >= chance {
                    continue;
                }
                let angle = rib as f32 * TAU / RIBS as f32;
                let tip = at.transform_point(Vec3::new(angle.cos(), -shape::rise(), angle.sin()));
                let light = (lamps.light_at(tip) / FULL_LIGHT).min(Vec3::splat(BRIGHTEST));
                drops.place(tip, now, light);
                shed = true;
            }
        }
        if shed && let Some(mut mesh) = meshes.get_mut(&drops.mesh) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, drops.anchors.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, drops.births.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, drops.lights.clone());
        }
    }

    fn place(&mut self, tip: Vec3, now: f32, light: Vec3) {
        let corners = self.next * 4..self.next * 4 + 4;
        self.anchors[corners.clone()].fill(tip.to_array());
        self.births[corners.clone()].fill([now, 0.0, 0.0]);
        self.lights[corners].fill(light.extend(1.0).to_array());
        self.next = (self.next + 1) % SLOTS;
    }
}

impl DropMaterial {
    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("drops.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl Material for DropMaterial {
    fn vertex_shader() -> ShaderRef {
        Self::shader()
    }

    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}
