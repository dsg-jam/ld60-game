import bevy_pbr::render::{
    mesh_view_bindings::{globals, view},
    view_transformations::position_world_to_clip,
};

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
    /// Seconds between drops, a random number, and nothing.
    @location(1) timing: vec3<f32>,
    @location(2) uv: vec2<f32>,
    /// The light the drop falls through, and its phase.
    @location(5) light: vec4<f32>,
}

struct Drop {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) thinning: f32,
}

/// Roughly the angle a pixel covers, in radians.
const PIXEL_ANGLE: f32 = 0.0012;

@vertex
fn vertex(vertex: Vertex) -> Drop {
    let period = vertex.timing.x;
    let t = fract(globals.time / period + vertex.light.w) * period;
    let fall = sqrt(2.0 * vertex.anchor.y / drips.gravity);
    let speed = drips.gravity * t;
    let head = vertex.anchor - vec3<f32>(0.0, 0.5 * drips.gravity * t * t, 0.0);
    let streak = clamp(speed * 0.03, 0.02, 0.3);
    let to_camera = view.world_position - head;
    let side = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), to_camera));
    let width = max(drips.width, length(to_camera) * PIXEL_ANGLE);
    let world = head + side * (vertex.uv.x - 0.5) * width + vec3<f32>(0.0, vertex.uv.y * streak, 0.0);
    var out: Drop;
    out.clip = position_world_to_clip(world);
    // Hidden once it hits the ground, or when it rains too little.
    if t > fall || vertex.timing.y >= drips.amount {
        out.clip = vec4<f32>(0.0);
    }
    out.uv = vertex.uv;
    out.color = vertex.light.rgb;
    out.thinning = drips.width / width;
    return out;
}

@fragment
fn fragment(in: Drop) -> @location(0) vec4<f32> {
    let across = 1.0 - abs(in.uv.x * 2.0 - 1.0);
    let along = 1.0 - in.uv.y * 0.7;
    return vec4<f32>(in.color * drips.brightness * across * along * in.thinning, 0.0);
}
