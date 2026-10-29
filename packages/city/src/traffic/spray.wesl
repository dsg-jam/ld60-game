import bevy_pbr::render::{
    mesh_functions::get_world_from_local,
    mesh_view_bindings::{globals, view},
    view_transformations::position_world_to_clip,
};

struct Spray {
    /// Millimeters of water under the front left, front right, rear left
    /// and rear right wheels.
    water: vec4<f32>,
    /// Seconds a droplet flies.
    lifetime: f32,
    size: f32,
    /// Where the tail lights sit, on the side of +Z, which light the spray
    /// red.
    tail: vec3<f32>,
    gravity: f32,
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
    @location(3) water: f32,
}

/// The water of a soaked road gives the mist of a wet road. Deeper water,
/// like a puddle, throws drops further, higher and thicker.
const FILM: f32 = 1.0;
const DEEP: vec2<f32> = vec2<f32>(3.0, 25.0);
const DEEP_THROW: f32 = 0.8;
const DEEP_SIZE: f32 = 0.6;
const DEEP_GLOW: f32 = 1.5;

/// The water under the wheel a droplet leaves.
fn water_under(anchor: vec3<f32>, side: f32) -> f32 {
    let front = anchor.x > 0.0;
    let right = side > 0.0;
    if front {
        return select(spray.water.x, spray.water.y, right);
    }
    return select(spray.water.z, spray.water.w, right);
}

@vertex
fn vertex(vertex: Vertex) -> Droplet {
    let r = vertex.random;
    let age = fract(globals.time / spray.lifetime + r.x);
    let t = age * spray.lifetime;
    let water = water_under(vertex.anchor, r.w);
    let deep = smoothstep(DEEP.x, DEEP.y, water);
    // Thrown back and up by the tire, then falling. Deep water goes higher
    // and out to the side.
    let throw = 1.0 + DEEP_THROW * deep;
    let velocity = vec3<f32>(
        -mix(1.5, 4.5, r.y),
        mix(0.6, 2.0, r.z) * throw,
        r.w * mix(0.2, 1.0, r.y) * throw * throw,
    );
    var local = vertex.anchor + velocity * t - vec3<f32>(0.0, 0.5 * spray.gravity * t * t, 0.0);
    local.y = max(local.y, 0.02);
    let world = (get_world_from_local(vertex.instance_index) * vec4<f32>(local, 1.0)).xyz;

    let to_camera = normalize(view.world_position - world);
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), to_camera));
    let up = cross(to_camera, right);
    let corner = (vertex.uv - 0.5) * spray.size * mix(0.3, 1.0, age) * (1.0 + DEEP_SIZE * deep);
    var out: Droplet;
    out.clip = position_world_to_clip(world + right * corner.x + up * corner.y);
    out.uv = vertex.uv;
    out.local = local;
    out.age = age;
    out.water = water;
    return out;
}

@fragment
fn fragment(in: Droplet) -> @location(0) vec4<f32> {
    let soft = 1.0 - smoothstep(0.0, 1.0, length(in.uv - 0.5) * 2.0);
    let amount = clamp(in.water / FILM, 0.0, 1.0) + DEEP_GLOW * smoothstep(DEEP.x, DEEP.y, in.water);
    let fade = (1.0 - in.age) * (1.0 - in.age) * amount * 0.4;
    let tail = spray.tail * vec3<f32>(1.0, 1.0, sign(in.local.z));
    let d = in.local - tail;
    let red = 1.0 / (1.0 + dot(d, d) * 4.0);
    let light = vec3<f32>(0.04, 0.045, 0.05) + vec3<f32>(1.0, 0.08, 0.05) * red * 1.5;
    return vec4<f32>(light * soft * fade, 0.0);
}
