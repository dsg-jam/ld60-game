/// Draws the density of a steam plume into a 3D texture, every frame.
import package::geometry::noise::pcg;

/// Matches `PlumeParams` in `steam.rs`.
struct Plume {
    radius: f32,
    height: f32,
    /// Of the box the plume fits in.
    width: f32,
    time: f32,
}

@group(0) @binding(0) var<uniform> plume: Plume;
@group(0) @binding(1) var density: texture_storage_3d<rgba8unorm, write>;

/// The plume widens by this much per meter it rises. Turbulent plumes take
/// in the air around them at a steady rate, after Morton, Taylor and Turner,
/// which makes them cones. Matches `SPREAD` in `steam.rs`.
const SPREAD: f32 = 0.12;
/// How far the plume sways sideways per meter it rises. Matches `SWAY` in
/// `steam.rs`.
const SWAY: f32 = 0.12;
/// Meters per second.
const RISE: f32 = 0.06;
/// The steam mixes into the cool air of the shop and vanishes over about
/// this height.
const FADE: f32 = 0.3;
/// Size of the wisps, in meters.
const WISP: f32 = 0.025;
/// How much the wisps twist and curl, in meters.
const CURL: f32 = 0.015;
const MAX_U32: f32 = 4294967295.0;

fn hash(cell: vec3<i32>) -> f32 {
    let bits = bitcast<vec3<u32>>(cell);
    return f32(pcg(bits.x ^ pcg(bits.y ^ pcg(bits.z)))) / MAX_U32;
}

/// Smooth noise from 0 to 1, changing about once per unit.
fn noise(p: vec3<f32>) -> f32 {
    let cell = vec3<i32>(floor(p));
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    let x00 = mix(hash(cell), hash(cell + vec3<i32>(1, 0, 0)), s.x);
    let x10 = mix(hash(cell + vec3<i32>(0, 1, 0)), hash(cell + vec3<i32>(1, 1, 0)), s.x);
    let x01 = mix(hash(cell + vec3<i32>(0, 0, 1)), hash(cell + vec3<i32>(1, 0, 1)), s.x);
    let x11 = mix(hash(cell + vec3<i32>(0, 1, 1)), hash(cell + vec3<i32>(1, 1, 1)), s.x);
    return mix(mix(x00, x10, s.y), mix(x01, x11, s.y), s.z);
}

@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(density);
    if any(id >= size) {
        return;
    }
    let t = plume.time;
    let voxel = vec3<f32>(plume.width, plume.height, plume.width) / vec3<f32>(size);
    let p = (vec3<f32>(id) + 0.5) * voxel - vec3<f32>(plume.width * 0.5, 0.0, plume.width * 0.5);
    let height = p.y;

    // The plume sways more the higher it gets.
    let axis = vec2<f32>(
        sin(t * 0.5 + height * 6.0) + 0.5 * sin(t * 1.3 + height * 11.0),
        sin(t * 0.4 + height * 7.0 + 2.0) + 0.5 * sin(t * 1.1 + height * 9.0),
    ) * SWAY * height / 1.5;
    let reach = plume.radius + SPREAD * height;
    // Thinner as it spreads and mixes, only forms just above the surface,
    // and gone by the top of the box.
    let top = 1.0 - height / plume.height;
    let thickness = plume.radius / reach
        * exp(-height / FADE)
        * min(height / (2.0 * voxel.y), 1.0)
        * min(top * 4.0, 1.0);
    let off_axis = length(p.xz - axis) / reach;
    let profile = exp(-2.0 * off_axis * off_axis);

    // Wisps rise with the steam, twisted by more noise, after Quilez,
    // "Domain Warping".
    let moving = (p - vec3<f32>(0.0, RISE * t, 0.0)) / WISP;
    let curl = vec3<f32>(noise(moving * 0.5 + 7.0), 0.0, noise(moving * 0.5 + 13.0)) - 0.5;
    let n = noise(moving + curl * (2.0 * CURL / WISP));
    let wispy = clamp((n - 0.45) / 0.2, 0.0, 1.0);

    let d = clamp(thickness * profile * wispy, 0.0, 1.0);
    textureStore(density, id, vec4<f32>(d, 0.0, 0.0, 1.0));
}
