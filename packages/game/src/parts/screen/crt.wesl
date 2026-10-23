import bevy_pbr::render::forward_io::VertexOutput;
import ld60_geometry::noise::{TAU, hash, noise};

/// Cells across the screen, each with a chance of a speck of dust.
const SPECKS: f32 = 120.0;
/// Dust lit by the room, on top of whatever the screen shows.
const DIRT: vec3<f32> = vec3<f32>(0.09, 0.085, 0.075);

struct Crt {
    // In UV units.
    corner_radius: f32,
    scanlines: f32,
    brightness: f32,
    scanline_darkness: f32,
    black_level: vec3<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var screen_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var screen_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> crt: Crt;

fn rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + radius;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

/// Dust and a smudge on the glass, which catch a little of the room's light.
fn dirt(uv: vec2<f32>) -> vec3<f32> {
    // A few specks of dust, mostly tiny.
    let grid = uv * SPECKS;
    let cell = floor(grid);
    let h = hash(cell, 3.0);
    let radius = mix(0.06, 0.4, h.w * h.w * h.w);
    let speck = step(0.994, h.x) * (1.0 - smoothstep(radius * 0.5, radius, length(fract(grid) - h.yz)));
    // A thumb smear near the lower right corner and haze towards the edges.
    let smear = smoothstep(0.55, 0.85, noise(uv * vec2<f32>(9.0, 5.0), 7.0))
        * (1.0 - smoothstep(0.0, 0.35, distance(uv, vec2<f32>(0.8, 0.75))));
    let haze = smoothstep(0.6, 1.0, length(uv * 2.0 - 1.0)) * noise(uv * 4.0, 9.0);
    return DIRT * (speck * 0.8 + smear * 0.25 + haze * 0.15);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(screen_texture, screen_sampler, in.uv).rgb;
    let centered = in.uv * 2.0 - 1.0;

    let edge = rounded_box(centered, vec2<f32>(1.0), crt.corner_radius * 2.0);
    let inside = 1.0 - smoothstep(-0.01, 0.0, edge);
    // Scanlines fade out when they get thinner than a few pixels, so they
    // don't alias into moire patterns from far away.
    let phase = in.uv.y * crt.scanlines;
    let pixels_per_line = 1.0 / max(fwidth(phase), 0.0001);
    let visibility = smoothstep(1.5, 3.0, pixels_per_line);
    let gap = 0.5 - 0.5 * cos(phase * TAU);
    let scanline = 1.0 - crt.scanline_darkness * visibility * gap;
    let vignette = smoothstep(1.7, 0.7, length(centered));

    // Unlit glass outside the picture, and the faint glow of the phosphor
    // inside it, so scanlines also show on black.
    let glass = vec3<f32>(0.006, 0.012, 0.008);
    let lit = (crt.black_level + color * crt.brightness) * scanline * vignette;
    return vec4<f32>(mix(glass, lit, inside) + dirt(in.uv), 1.0);
}
