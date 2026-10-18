import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_render::view::View;

/// Matches `HeightFog` in `height_fog.rs`.
struct HeightFog {
    color: vec4<f32>,
    density: f32,
    falloff: f32,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var depth_texture: texture_depth_2d;
@group(0) @binding(2) var screen: texture_2d<f32>;
@group(0) @binding(3) var screen_sampler: sampler;
@group(0) @binding(4) var<uniform> fog: HeightFog;

/// The sky counts as this far away.
const SKY_DISTANCE: f32 = 10000.0;
/// Keeps `exp` from overflowing for rays that head down forever.
const MAX_EXPONENT: f32 = 80.0;

/// How much of the haze a ray collects on its way through air that thins
/// out with height, after Quilez, "Better Fog".
fn haze_along(origin: vec3<f32>, ray: vec3<f32>, distance: f32) -> f32 {
    let thickness = fog.density * exp(-fog.falloff * origin.y);
    let rise = ray.y * fog.falloff;
    if abs(rise * distance) < 1e-4 {
        return thickness * distance;
    }
    return thickness * (1.0 - exp(min(-rise * distance, MAX_EXPONENT))) / rise;
}

fn world_at(ndc: vec2<f32>, depth: f32) -> vec3<f32> {
    let p = view.world_from_clip * vec4<f32>(ndc, depth, 1.0);
    return p.xyz / p.w;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(screen, screen_sampler, in.uv);
    let depth = textureLoad(depth_texture, vec2<i32>(in.position.xy), 0);
    let ndc = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let origin = view.world_position;
    // Depth is reversed: the near plane is at 1 and the sky at 0.
    let ray = normalize(world_at(ndc, 1.0) - origin);
    var distance = SKY_DISTANCE;
    if depth > 0.0 {
        distance = length(world_at(ndc, depth) - origin);
    }
    let haze = 1.0 - exp(-haze_along(origin, ray, distance));
    return vec4<f32>(mix(color.rgb, fog.color.rgb, haze), color.a);
}
