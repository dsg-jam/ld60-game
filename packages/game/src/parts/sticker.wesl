import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;
@if(PREPASS_PIPELINE)
import bevy_pbr::{
    deferred::functions::deferred_output,
    prepass::io::{VertexOutput, FragmentOutput},
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};
import package::geometry::noise::{PI, TAU, hash, noise};

struct Sticker {
    size: vec2<f32>,
    seed: f32,
    wear: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> sticker: Sticker;

const PAPER: vec3<f32> = vec3<f32>(0.85, 0.82, 0.74);
const DIRT: vec3<f32> = vec3<f32>(0.3, 0.24, 0.17);
const COFFEE: vec3<f32> = vec3<f32>(0.45, 0.3, 0.16);
const GREASE: vec3<f32> = vec3<f32>(0.5, 0.48, 0.42);
/// Old paper turns yellow.
const AGED: vec3<f32> = vec3<f32>(1.0, 0.95, 0.82);
const SCRATCHES: i32 = 5;

/// How much `p` lies on a thin scratch from `a` to `b`. All are in meters.
fn scratch(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, width: f32) -> f32 {
    let ab = b - a;
    let t = clamp(dot(p - a, ab) / dot(ab, ab), 0.0, 1.0);
    let d = length(p - a - ab * t);
    // Scratches start and end lightly.
    let taper = sin(t * PI);
    return (1.0 - smoothstep(0.0, width, d)) * taper;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let size = sticker.size;
    let s = sticker.seed;
    let w = sticker.wear;
    // Meters from the top left corner.
    let p = in.uv * size;
    var color = pbr_input.material.base_color.rgb;
    var rough = pbr_input.material.perceptual_roughness;

    // Faded print and paper fibers.
    color = mix(color, PAPER, 0.12 * w * (0.6 + 0.8 * noise(p * 60.0, s)));
    color *= 0.96 + 0.06 * noise(p * 2500.0, s + 1.0);
    color *= mix(vec3<f32>(1.0), AGED, w);

    // Dirt worked in along the edges, where fingers and dust catch.
    let edge = min(min(p.x, size.x - p.x), min(p.y, size.y - p.y));
    let reach = 0.0015 + 0.003 * noise(vec2<f32>(p.x + p.y, 0.5) * 300.0, s + 2.0);
    let grime = (1.0 - smoothstep(0.0, reach, edge)) * w;
    color = mix(color, DIRT, grime * 0.55);
    rough = mix(rough, 0.85, grime);

    // A coffee ring, partly on the sticker.
    let h = hash(vec2<f32>(s, 3.0), 50.0);
    let center = size * vec2<f32>(h.x * 1.4 - 0.2, h.y * 1.4 - 0.2);
    let radius = mix(0.008, 0.014, h.z);
    let r = length(p - center) + 0.0006 * noise(p * 400.0, s + 3.0);
    let ring = exp(-pow((r - radius) / 0.0005, 2.0)) * (0.5 + 0.5 * noise(p * 150.0, s + 4.0));
    let inside = (1.0 - smoothstep(radius - 0.001, radius, r)) * 0.25;
    let coffee = clamp(ring + inside, 0.0, 1.0) * step(0.3, h.w) * w;
    color = mix(color, color * COFFEE * 1.6, coffee * 0.7);

    // Greasy smudges from thumbs.
    let smudge = smoothstep(0.55, 0.85, noise(p * 90.0, s + 5.0) * 0.7 + noise(p * 300.0, s + 6.0) * 0.3) * w;
    color = mix(color, GREASE * color * 1.4, smudge * 0.35);
    rough = mix(rough, 0.3, smudge);

    // Scratches through the print down to the paper.
    var scratched = 0.0;
    for (var i = 0; i < SCRATCHES; i++) {
        let k = hash(vec2<f32>(s, f32(i)), 60.0);
        let a = size * k.xy;
        let angle = k.z * TAU;
        let span = mix(0.005, 0.03, k.w);
        let b = a + vec2<f32>(cos(angle), sin(angle)) * span;
        scratched = max(scratched, scratch(p, a, b, 0.00015));
    }
    scratched *= w;
    color = mix(color, PAPER * 0.95, scratched * 0.8);
    rough = mix(rough, 0.9, scratched);

    pbr_input.material.base_color = vec4<f32>(color, 1.0);
    pbr_input.material.perceptual_roughness = rough;

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
