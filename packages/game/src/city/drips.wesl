import bevy_pbr::render::{
    mesh_view_bindings::{globals, view},
    view_transformations::position_world_to_clip,
};
import package::geometry::streak::{across, streak};

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
    /// Seconds between drops, a random number, and its phase.
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
    let t = fract(globals.time / period + vertex.timing.z) * period;
    let fall = sqrt(2.0 * vertex.anchor.y / drips.gravity);
    let speed = drips.gravity * t;
    let head = vertex.anchor - vec3<f32>(0.0, 0.5 * drips.gravity * t * t, 0.0);
    // Longer the faster it falls, trailing above the head.
    let tail = vec3<f32>(0.0, clamp(speed * 0.03, 0.02, 0.3), 0.0);
    let shape = streak(head, tail, drips.width, vertex.uv, view.world_position);
    var out: Drop;
    out.clip = position_world_to_clip(shape.world);
    // Hidden once it hits the ground, or when it rains too little.
    if t > fall || vertex.timing.y >= drips.amount {
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
