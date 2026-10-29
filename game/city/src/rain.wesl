import bevy_pbr::render::{
    mesh_view_bindings::{globals, view},
    view_transformations::position_world_to_clip,
};
import ld60_geometry::streak::{across, streak};

struct Rain {
    /// Corner of the box the rain falls in.
    origin: vec3<f32>,
    width: f32,
    size: vec3<f32>,
    length: f32,
    fall: vec3<f32>,
    ambient: f32,
    /// Share of the streaks that fall, from 0 to 1.
    amount: f32,
    /// Position and strength of the lights that make the rain visible.
    lights: array<vec4<f32>, LIGHTS>,
    colors: array<vec4<f32>, LIGHTS>,
}

/// Matches `LIGHTS` in `rain.rs`.
const LIGHTS: u32 = 8u;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> rain: Rain;

struct Vertex {
    /// The same random point for all corners of a streak.
    @location(0) seed: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct Falling {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world: vec3<f32>,
    /// Streaks thinner than a pixel get wider and fainter instead.
    @location(2) thinning: f32,
}

@vertex
fn vertex(vertex: Vertex) -> Falling {
    let drop = rain.origin + fract(vertex.seed + rain.fall * globals.time / rain.size) * rain.size;
    let along = normalize(rain.fall) * rain.length;
    let shape = streak(drop - along * 0.5, along, rain.width, vertex.uv, view.world_position);
    var out: Falling;
    out.clip = position_world_to_clip(shape.world);
    // Light rain drops some of the streaks.
    if fract(vertex.seed.x * 13.7 + vertex.seed.z * 7.1) >= rain.amount {
        out.clip = vec4<f32>(0.0);
    }
    out.uv = vertex.uv;
    out.world = shape.world;
    out.thinning = shape.thinning;
    return out;
}

@fragment
fn fragment(in: Falling) -> @location(0) vec4<f32> {
    let along = smoothstep(0.0, 0.3, in.uv.y) * (1.0 - in.uv.y * 0.6);
    // Rain only shows where light falls on it.
    var light = vec3<f32>(0.6, 0.7, 1.0) * rain.ambient;
    for (var i = 0u; i < LIGHTS; i++) {
        let lamp = rain.lights[i];
        let d = in.world - lamp.xyz;
        light += rain.colors[i].rgb * lamp.w / (1.0 + dot(d, d) * 0.25);
    }
    let fade = 1.0 - smoothstep(12.0, 25.0, distance(in.world, view.world_position));
    return vec4<f32>(light * across(in.uv) * along * fade * in.thinning, 0.0);
}
