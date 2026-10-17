import bevy_pbr::render::forward_io::VertexOutput;

struct Crt {
    // In UV units.
    corner_radius: f32,
    scanlines: f32,
    brightness: f32,
    scanline_darkness: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var screen_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var screen_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> crt: Crt;

fn rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + radius;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
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
    let gap = 0.5 - 0.5 * cos(phase * 6.28318531);
    let scanline = 1.0 - crt.scanline_darkness * visibility * gap;
    let vignette = smoothstep(1.7, 0.7, length(centered));

    // Unlit glass outside the picture, and the faint glow of the phosphor
    // inside it, so scanlines also show on black.
    let glass = vec3<f32>(0.006, 0.012, 0.008);
    let black_level = vec3<f32>(0.02, 0.045, 0.03);
    let lit = (black_level + color * crt.brightness) * scanline * vignette;
    return vec4<f32>(mix(glass, lit, inside), 1.0);
}
