use std::f32::consts::TAU;

use bevy::prelude::*;
use ld60_geometry::triangles::Triangles;

/// Steam rises this high over a dish this wide.
const HEIGHT: f32 = 0.2;
const WIDTH: f32 = 0.02;
/// Ribbons of steam, crossed so some are seen from any side.
const RIBBONS: usize = 3;
const STEPS: usize = 12;
/// How often the steam turns and breathes, per second.
const TURN: f32 = 0.5;
const BREATH: f32 = 1.3;
const COLOR: Color = Color::srgba(0.82, 0.8, 0.76, 0.09);

/// Steam rising from hot food, turning and swelling slowly.
#[derive(Component, Debug)]
pub(in super::super) struct Steam {
    phase: f32,
    /// How many times [`WIDTH`] wide it is.
    size: f32,
}

impl Steam {
    /// Steam over food `radius` wide.
    pub(in super::super) fn over(radius: f32, phase: f32) -> Self {
        Self {
            phase,
            size: radius / WIDTH,
        }
    }

    /// Thin curling ribbons, soft at their edges, faded in at the bottom
    /// and out at the top.
    pub(in super::super) fn mesh() -> Mesh {
        let mut mesh = Triangles::default();
        for ribbon in 0..RIBBONS {
            let turn = TAU * ribbon as f32 / RIBBONS as f32;
            let across = Quat::from_rotation_y(turn) * Vec3::X;
            let out = Quat::from_rotation_y(turn) * Vec3::Z;
            let mut rows = Vec::with_capacity(STEPS + 1);
            for step in 0..=STEPS {
                let t = step as f32 / STEPS as f32;
                let curl = WIDTH * t * (t * 9.0 + turn).sin();
                let middle = Vec3::Y * HEIGHT * t + across * curl + out * 0.006 * t;
                let half = WIDTH * (0.2 + 0.7 * t);
                let fade = (t / 0.25).min(1.0) * (1.0 - t).powf(1.5);
                let mut vertex = |at: Vec3, alpha: f32| {
                    mesh.set_color([1.0, 1.0, 1.0, alpha]);
                    mesh.vertex(at, out)
                };
                rows.push([
                    vertex(middle - across * half, 0.0),
                    vertex(middle, fade),
                    vertex(middle + across * half, 0.0),
                ]);
            }
            for pair in rows.windows(2) {
                let [near, far] = [pair[0], pair[1]];
                for side in 0..2 {
                    mesh.triangle([near[side], near[side + 1], far[side + 1]]);
                    mesh.triangle([near[side], far[side + 1], far[side]]);
                }
            }
        }
        mesh.build()
    }

    pub(in super::super) fn material() -> StandardMaterial {
        StandardMaterial {
            base_color: COLOR,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        }
    }

    pub(in super::super) fn drift(
        time: Res<Time>,
        mut steams: Query<(&Steam, &InheritedVisibility, &mut Transform)>,
    ) {
        let now = time.elapsed_secs();
        for (steam, visible, mut transform) in &mut steams {
            if !visible.get() {
                continue;
            }
            let breath = (now * BREATH + steam.phase).sin();
            transform.rotation = Quat::from_rotation_y(now * TURN + steam.phase);
            let wide = steam.size * (1.0 + 0.15 * breath);
            transform.scale = Vec3::new(wide, 1.0 + 0.1 * breath, wide);
        }
    }
}
