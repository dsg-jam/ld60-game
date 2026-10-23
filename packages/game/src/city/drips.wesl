import bevy_pbr::render::{
    mesh_view_bindings::{globals, view},
    view_transformations::position_world_to_clip,
};
import ld60_geometry::{
    noise::hash1,
    streak::{across, streak},
};

/// Salts of the random numbers of each fall, as in `drips.rs`.
const CHANCE: f32 = 1.0;
const WAIT: f32 = 2.0;

struct Drips {
    /// Share of the drops that fall, from 0 to 1.
    amount: f32,
    width: f32,
    brightness: f32,
    gravity: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> drips: Drips;

struct Vertex {
    /// Where the drop hangs before it falls.
    @location(0) anchor: vec3<f32>,
    /// Seconds in a round of the drip, a whole number that picks the random
    /// numbers of each round, and its phase.
    @location(1) timing: vec3<f32>,
    @location(2) uv: vec2<f32>,
    /// The light the drop falls through.
    @location(5) light: vec4<f32>,
}

struct Drop {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) thinning: f32,
}

@vertex
fn vertex(vertex: Vertex) -> Drop {
    let period = vertex.timing.x;
    let fall = sqrt(2.0 * vertex.anchor.y / drips.gravity);
    // Each round the drop falls at a random time, or not at all.
    let rounds = globals.time / period + vertex.timing.z;
    let round = vec2<f32>(floor(rounds), vertex.timing.y);
    let wait = hash1(round, WAIT) * (period - fall);
    let t = fract(rounds) * period - wait;
    let speed = drips.gravity * t;
    let head = vertex.anchor - vec3<f32>(0.0, 0.5 * drips.gravity * t * t, 0.0);
    // Longer the faster it falls, trailing above the head.
    let tail = vec3<f32>(0.0, clamp(speed * 0.03, 0.02, 0.3), 0.0);
    let shape = streak(head, tail, drips.width, vertex.uv, view.world_position);
    var out: Drop;
    out.clip = position_world_to_clip(shape.world);
    // Hidden before it falls, once it hits the ground, and in the rounds it
    // skips, more of them the less it rains.
    if t < 0.0 || t > fall || hash1(round, CHANCE) >= drips.amount {
        out.clip = vec4<f32>(0.0);
    }
    out.uv = vertex.uv;
    out.color = vertex.light.rgb;
    out.thinning = shape.thinning;
    return out;
}

@fragment
fn fragment(in: Drop) -> @location(0) vec4<f32> {
    let along = 1.0 - in.uv.y * 0.7;
    return vec4<f32>(in.color * drips.brightness * across(in.uv) * along * in.thinning, 0.0);
}
