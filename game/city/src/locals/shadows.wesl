import bevy_pbr::render::view_transformations::position_world_to_clip;

/// Match `shadows.rs`.
const CASTERS: u32 = 40u;
const LIGHTS: u32 = 3u;
const BLOBS: u32 = 3u;
const BONES: u32 = 7u;
/// The least blur of a shadow edge, in meters.
const BLUR: f32 = 0.02;
/// Above the ground, against flickering.
const LIFT: f32 = 0.006;

struct Caster {
    bounds: vec4<f32>,
    ground: vec4<f32>,
    lights: array<vec4<f32>, LIGHTS>,
    tints: array<vec4<f32>, LIGHTS>,
    /// Per ellipsoid: its center with its mean radius, then the rows that
    /// take a point into the space where it is a unit sphere.
    blobs: array<vec4<f32>, 12>,
    /// Per capsule: one end with its radius, then the other end.
    bones: array<vec4<f32>, 14>,
}

struct Casters {
    casters: array<Caster, CASTERS>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> shadows: Casters;

struct Vertex {
    @location(2) uv: vec2<f32>,
    /// The caster's index in red.
    @location(5) index: vec4<f32>,
}

struct Ground {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) @interpolate(flat) index: u32,
}

@vertex
fn vertex(vertex: Vertex) -> Ground {
    let index = u32(vertex.index.x);
    let caster = shadows.casters[index];
    // Flipped along Z to face up.
    let corner = mix(caster.bounds.xy, caster.bounds.zw, vec2<f32>(vertex.uv.x, 1.0 - vertex.uv.y));
    let world = vec3<f32>(corner.x, caster.ground.x + LIFT, corner.y);
    var out: Ground;
    out.clip = position_world_to_clip(world);
    out.world = world;
    out.index = index;
    return out;
}

/// How much of a light a round part of `size` hides when the ray passes
/// `gap` outside of it, where the light's cone is `cone` wide.
fn hidden(gap: f32, size: f32, cone: f32) -> f32 {
    return (1.0 - smoothstep(-cone, cone, gap)) * min(size / cone, 1.0);
}

/// The ray from `start` along `along` for `reach` meters, past an
/// ellipsoid.
fn past_blob(caster: Caster, blob: u32, start: vec3<f32>, along: vec3<f32>, reach: f32, spread: f32) -> f32 {
    let center = caster.blobs[blob * 4u];
    if center.w <= 0.0 {
        return 0.0;
    }
    let rows = mat3x3<f32>(
        caster.blobs[blob * 4u + 1u].xyz,
        caster.blobs[blob * 4u + 2u].xyz,
        caster.blobs[blob * 4u + 3u].xyz,
    );
    // Rows as columns: transposed, to take vectors into the unit sphere.
    let p = (start - center.xyz) * rows;
    let d = along * rows;
    let t = clamp(-dot(p, d) / dot(d, d), 0.0, reach);
    let gap = (length(p + d * t) - 1.0) * center.w;
    return hidden(gap, center.w, max(spread * t, BLUR));
}

/// The ray past a capsule.
fn past_bone(caster: Caster, bone: u32, start: vec3<f32>, along: vec3<f32>, reach: f32, spread: f32) -> f32 {
    let a = caster.bones[bone * 2u];
    let b = caster.bones[bone * 2u + 1u].xyz;
    let radius = a.w;
    if radius <= 0.0 {
        return 0.0;
    }
    let axis = b - a.xyz;
    let w = start - a.xyz;
    let k = dot(along, axis);
    let c = dot(axis, axis);
    let e = dot(axis, w);
    let f = dot(along, w);
    let denominator = c - k * k;
    var t = clamp(select((k * e - c * f) / denominator, 0.0, denominator < 1e-6), 0.0, reach);
    let s = clamp((k * t + e) / c, 0.0, 1.0);
    t = clamp(dot(a.xyz + axis * s - start, along), 0.0, reach);
    let gap = distance(start + along * t, a.xyz + axis * s) - radius;
    return hidden(gap, radius, max(spread * t, BLUR));
}

@fragment
fn fragment(in: Ground) -> @location(0) vec4<f32> {
    let caster = shadows.casters[in.index];
    var lit = vec3<f32>(1.0);
    for (var light = 0u; light < LIGHTS; light++) {
        let tint = caster.tints[light].rgb;
        if all(tint == vec3<f32>(0.0)) {
            continue;
        }
        let lamp = caster.lights[light];
        let to = lamp.xyz - in.world;
        let reach = length(to);
        let along = to / reach;
        let spread = lamp.w / reach;
        var through = 1.0;
        for (var blob = 0u; blob < BLOBS; blob++) {
            through *= 1.0 - past_blob(caster, blob, in.world, along, reach, spread);
        }
        for (var bone = 0u; bone < BONES; bone++) {
            through *= 1.0 - past_bone(caster, bone, in.world, along, reach, spread);
        }
        lit -= tint * (1.0 - through);
    }
    return vec4<f32>(max(lit, vec3<f32>(0.0)), 1.0);
}
