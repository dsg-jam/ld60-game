/// The tone of a sheet: the paper its print run went onto, and how it has
/// aged. The run decides the paper and the pattern it yellows in. Each
/// sheet differs a little in how far it yellowed, how dark its edges went
/// and which side the sun faded.

import ld60_geometry::noise::{TAU, hash, noise, specks};

/// Paper yellowed all the way.
const YELLOWED: vec3<f32> = vec3<f32>(1.0, 0.86, 0.62);
/// The brown spots of foxing on old paper.
const FOXED: vec3<f32> = vec3<f32>(0.82, 0.64, 0.45);

/// The tone of the bare paper at `p`, in meters from the top left of a
/// sheet of `size`. `stock` is the tint of the fresh paper and in `w` its
/// age. `run` picks what the print run shares and `s` the sheet.
fn toned(p: vec2<f32>, size: vec2<f32>, stock: vec4<f32>, run: f32, s: f32, pixel: f32) -> vec3<f32> {
    let sheet = hash(vec2<f32>(run, s), 90.0);
    let age = stock.w * mix(0.75, 1.25, sheet.x);
    // Broad clouds, in a pattern of the run's own.
    let clouds = noise(p * 6.0, run + 91.0) * 0.6 + noise(p * 17.0, s + 92.0) * 0.4;
    var yellow = age * (0.25 + 0.35 * clouds);
    // Edges brown first, where air and hands reach.
    let edge = min(min(p.x, size.x - p.x), min(p.y, size.y - p.y));
    let reach = mix(0.004, 0.014, sheet.y) * (0.7 + 0.6 * noise(vec2<f32>(p.x + p.y, 0.5) * 60.0, s + 93.0));
    yellow += age * 0.45 * (1.0 - smoothstep(0.0, reach, edge));
    // One side lay in the sun for longer.
    let toward = vec2<f32>(cos(sheet.z * TAU), sin(sheet.z * TAU));
    let sunned = clamp(dot(p / size - 0.5, toward) + 0.5, 0.0, 1.0);
    yellow += age * 0.25 * sunned;
    var tone = stock.rgb * (0.98 + 0.04 * sheet.w) * mix(vec3<f32>(1.0), YELLOWED, clamp(yellow, 0.0, 1.0) * 0.7);
    let foxing = specks(p, 0.012, 0.3 * smoothstep(0.5, 0.9, age), 0.0003, 0.0012, s + 94.0, pixel);
    return mix(tone, tone * FOXED, foxing * 0.6);
}
