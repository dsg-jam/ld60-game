import bevy_pbr::render::{
    mesh_view_bindings::{globals, view},
    view_transformations::position_world_to_clip,
};
import ld60_geometry::streak::{across, streak};

struct Drops {
    width: f32,
    gravity: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> drops: Drops;

struct Vertex {
    /// The rib tip the drop left.
    @location(0) anchor: vec3<f32>,
    /// When it left, in the time of the globals.
    @location(1) birth: vec3<f32>,
    @location(2) uv: vec2<f32>,
    /// The light it falls through.
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
    let t = globals.time - vertex.birth.x;
    let fall = sqrt(2.0 * max(vertex.anchor.y, 0.0) / drops.gravity);
    let speed = drops.gravity * t;
    let head = vertex.anchor - vec3<f32>(0.0, 0.5 * drops.gravity * t * t, 0.0);
    let tail = vec3<f32>(0.0, clamp(speed * 0.012, 0.004, 0.06), 0.0);
    let shape = streak(head, tail, drops.width, vertex.uv, view.world_position);
    var out: Drop;
    out.clip = position_world_to_clip(shape.world);
    if t < 0.0 || t > fall {
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
    return vec4<f32>(in.color * across(in.uv) * along * in.thinning, 0.0);
}
