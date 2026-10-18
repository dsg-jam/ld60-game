import bevy_pbr::render::{
    mesh_functions::get_world_from_local,
    mesh_view_bindings::{globals, view},
    view_transformations::position_world_to_clip,
};

struct Spray {
    /// From 0 for a dry road to 1 for soaked.
    wetness: f32,
    /// Seconds a droplet flies.
    lifetime: f32,
    size: f32,
    /// Local X of the tail lights, which light the spray red.
    tail: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> spray: Spray;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    /// Where the droplet leaves the tire, in the car's space.
    @location(0) anchor: vec3<f32>,
    @location(2) uv: vec2<f32>,
    /// Phase, two random numbers and the side of the car.
    @location(5) random: vec4<f32>,
}

struct Droplet {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) local: vec3<f32>,
    @location(2) age: f32,
}

@vertex
fn vertex(vertex: Vertex) -> Droplet {
    let r = vertex.random;
    let age = fract(globals.time / spray.lifetime + r.x);
    let t = age * spray.lifetime;
    // Thrown back and up by the tire, then falling.
    let velocity = vec3<f32>(-mix(1.5, 4.5, r.y), mix(0.6, 2.0, r.z), r.w * mix(0.2, 1.0, r.y));
    var local = vertex.anchor + velocity * t - vec3<f32>(0.0, 4.9 * t * t, 0.0);
    local.y = max(local.y, 0.02);
    let world = (get_world_from_local(vertex.instance_index) * vec4<f32>(local, 1.0)).xyz;

    let to_camera = normalize(view.world_position - world);
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), to_camera));
    let up = cross(to_camera, right);
    let corner = (vertex.uv - 0.5) * spray.size * mix(0.3, 1.0, age);
    var out: Droplet;
    out.clip = position_world_to_clip(world + right * corner.x + up * corner.y);
    out.uv = vertex.uv;
    out.local = local;
    out.age = age;
    return out;
}

@fragment
fn fragment(in: Droplet) -> @location(0) vec4<f32> {
    let soft = 1.0 - smoothstep(0.0, 1.0, length(in.uv - 0.5) * 2.0);
    let fade = (1.0 - in.age) * (1.0 - in.age) * spray.wetness * 0.4;
    let tail = vec3<f32>(spray.tail, 0.65, sign(in.local.z) * 0.8);
    let d = in.local - tail;
    let red = 1.0 / (1.0 + dot(d, d) * 4.0);
    let light = vec3<f32>(0.04, 0.045, 0.05) + vec3<f32>(1.0, 0.08, 0.05) * red * 1.5;
    return vec4<f32>(light * soft * fade, 0.0);
}
