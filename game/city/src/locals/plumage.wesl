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
import ld60_geometry::{
    bump::bumped,
    noise::{PI, hash1, noise, visible},
};

struct Plumage {
    back: vec4<f32>,
    pale: vec4<f32>,
    dark: vec4<f32>,
    /// How soaked the feathers are, from 0 to 1.
    wet: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> plumage: Plumage;

/// Matches `Kind` in `plumage/feathers.rs`.
const PLUME: f32 = 1.0;
const DOWN: f32 = 2.0;
/// Matches `Region` in `plumage/feathers.rs`.
const PALE: f32 = 1.0;
const DARK: f32 = 2.0;
const PRIMARY: f32 = 3.0;
/// Down is a little darker than the feathers over it.
const DOWN_SHADE: f32 = 0.85;
/// Matches `CELLS` in `plumage/feathers.rs`.
const CELLS: f32 = 40.0;
/// Barbs across the vane of a feather.
const BARBS: f32 = 70.0;
/// Barbs sweep towards the tip.
const SWEEP: f32 = 0.7;

/// Half the width of a vane at `along` from the root at 0 to the tip at 1,
/// as a share of the card. Contour feathers are broad and round, plumes
/// narrow and pointed.
fn vane(along: f32, plume: bool) -> f32 {
    let t = clamp(along, 0.0, 1.0);
    if plume {
        return mix(0.35, 0.9, smoothstep(0.0, 0.3, t)) * pow(max(1.0 - t, 0.0), 0.7);
    }
    return sqrt(max(sin(PI * min(t * 0.85 + 0.12, 1.0)), 0.0));
}

/// The color of `region`, `along` a feather.
fn region_color(region: f32, along: f32) -> vec3<f32> {
    if region > PRIMARY - 0.5 {
        return mix(plumage.back.rgb, plumage.dark.rgb, smoothstep(0.35, 0.9, along));
    }
    if region > DARK - 0.5 {
        return plumage.dark.rgb;
    }
    if region > PALE - 0.5 {
        return plumage.pale.rgb;
    }
    return plumage.back.rgb;
}

/// Rows of small rounded feathers on down, `cells` of them around and
/// along, each a little darker at its root under the row before. Fades
/// out where they get too small to see.
fn shingles(uv: vec2<f32>, cells: vec2<f32>) -> f32 {
    let cell = uv * cells;
    let row = floor(cell.y);
    let x = fract(cell.x + 0.5 * (row - 2.0 * floor(row / 2.0)));
    let edge = fract(cell.y + 0.35 * sin(PI * x));
    let size = max(length(fwidth(cell)), 1e-4);
    let fade = 1.0 - smoothstep(0.15, 0.5, size);
    return mix(1.0, mix(0.9, 1.0, smoothstep(0.0, 0.7, edge)), fade);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let uv = in.uv;
    let across = uv.x * 2.0 - 1.0;
    let along = uv.y;
    let shade = in.color.r;
    let code = round(in.color.g * 15.0);
    let kind = floor(code / 4.0 + 0.01);
    let region = code - kind * 4.0;
    let seed = in.color.b * 97.0;
    let wet = plumage.wet;
    let down = kind > DOWN - 0.5;
    let plume = abs(kind - PLUME) < 0.5;

    // Wet barbs clump together into a few strands, with gaps between.
    let gaps = mix(0.0, 0.35, wet) * step(0.5, hash1(vec2<f32>(seed, 3.0), 2.0));
    let notch = noise(vec2<f32>(along * 9.0, seed), 4.0);
    let width = vane(along, plume) * (1.0 - gaps * step(0.55, notch) * step(0.4, abs(across)));
    if !down && (abs(across) > width || along > 0.995) {
        discard;
    }

    let pixel = length(fwidth(in.world_position.xyz));
    let barb_count = BARBS * mix(1.0, 0.35, wet);
    let barb = fract((along + abs(across) * SWEEP) * barb_count);
    let barb_height = 0.0001 * (1.0 - abs(barb * 2.0 - 1.0)) * visible(0.003, pixel) * select(1.0, 0.0, down);
    let rachis = 1.0 - smoothstep(0.03, 0.07, abs(across));

    var color = region_color(region, along) * shade;
    if down {
        color = region_color(region, 0.0) * DOWN_SHADE * shingles(uv, vec2<f32>(in.color.b, in.color.r) * CELLS);
    } else {
        // A little darker towards the root, where the next row hides it.
        color *= mix(0.85, 1.0, smoothstep(0.0, 0.5, along));
    }
    // Only plumes show their shafts.
    let shaft = rachis * select(0.0, 1.0, plume);
    color = mix(color, color * 1.2 + 0.02, shaft * 0.3);
    // Water darkens the barbs and leaves a film on them.
    color *= mix(1.0, 0.6, wet);
    var rough = mix(0.8, 0.4, wet);
    rough = mix(rough, rough * 0.8, shaft);

    pbr_input.material.base_color = vec4<f32>(color, 1.0);
    pbr_input.material.perceptual_roughness = rough;
    pbr_input.material.reflectance = vec3<f32>(mix(0.25, 0.5, wet));
    pbr_input.N = bumped(pbr_input.N, in.world_position.xyz, barb_height + 0.0002 * shaft);

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
