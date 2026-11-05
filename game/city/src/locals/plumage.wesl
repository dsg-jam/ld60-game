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
    lore: vec4<f32>,
    cloth: vec4<f32>,
    /// How soaked the feathers are, from 0 to 1.
    wet: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> plumage: Plumage;

/// Matches `Kind` in `plumage/feathers.rs`.
const PLUME: f32 = 1.0;
const DOWN: f32 = 2.0;
const CLOTH: f32 = 3.0;
/// Matches `Region` in `plumage/feathers.rs`.
const PALE: f32 = 1.0;
const DARK: f32 = 2.0;
const PRIMARY: f32 = 3.0;
const FACE: f32 = 4.0;
const LORE: f32 = 5.0;
/// Down is a little darker than the feathers over it.
const DOWN_SHADE: f32 = 0.85;
/// Matches `CELLS` in `plumage/feathers.rs`.
const CELLS: f32 = 100.0;
/// Where the eye is on the head, along its UV from the back of the head to
/// the tip of the bill and how far above the side, in radians, how long
/// the head is and how tall about the eye. Matches `bird/head.rs`.
const EYE_ALONG: f32 = 0.2322;
const EYE_RISE: f32 = 0.3;
const HEAD_LENGTH: f32 = 0.267;
const HEAD_HEIGHT: f32 = 0.03;
/// How far a small feather of the down stands out at its tip, in meters.
const SHINGLE_HEIGHT: f32 = 0.0012;
/// How much the fine barbs brighten edges seen at a low angle, dry.
const SHEEN: f32 = 0.3;
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

/// The face at `uv` on the head: pale, with a dark stripe from above the
/// eye back to the crest, and bare skin around the eye and before it.
fn face(uv: vec2<f32>) -> vec3<f32> {
    let rise = asin(sin(uv.x * 2.0 * PI));
    let along = (uv.y - EYE_ALONG) * HEAD_LENGTH;
    let up = (rise - EYE_RISE) * HEAD_HEIGHT;
    let behind = smoothstep(0.004, -0.004, along);
    // The stripe starts above the eye and widens towards the nape.
    let low = 0.007 - 0.004 * behind;
    let high = 0.016 + 0.01 * behind * smoothstep(0.0, -0.03, along);
    let stripe = smoothstep(low - 0.002, low + 0.002, up) * smoothstep(high + 0.002, high - 0.002, up)
        * smoothstep(0.012, 0.004, along);
    let ring = smoothstep(0.0125, 0.0105, length(vec2<f32>(along, up)));
    let before = smoothstep(0.0, 0.01, along) * smoothstep(0.012, 0.004, abs(up + 0.002));
    var color = mix(plumage.pale.rgb, plumage.dark.rgb, stripe);
    return mix(color, plumage.lore.rgb, max(ring, before));
}

/// A knitted surface: rows of stitches like small V shapes, `cells` of
/// them around and along. Fades out where they get too small to see.
fn knit(uv: vec2<f32>, cells: vec2<f32>) -> f32 {
    let cell = uv * cells;
    let x = fract(cell.x);
    let stitch = fract(cell.y + abs(x - 0.5) * 0.9);
    let ridge = 1.0 - pow(abs(stitch * 2.0 - 1.0), 2.0);
    let gap = smoothstep(0.08, 0.0, abs(x - 0.5));
    let size = max(length(fwidth(cell)), 1e-4);
    let fade = 1.0 - smoothstep(0.2, 0.6, size);
    return mix(0.85, mix(0.7, 1.0, ridge) * (1.0 - 0.2 * gap), fade);
}

/// The color of `region`, `along` a feather.
fn region_color(region: f32, along: f32) -> vec3<f32> {
    if region > LORE - 0.5 {
        return plumage.lore.rgb;
    }
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
/// along. Each lies over the root of the one behind it, so the surface
/// rises along a feather and steps down at its tip, and each has a shade
/// of its own. Returns the shade and the height in meters. Fades out where
/// they get too small to see.
fn shingles(uv: vec2<f32>, cells: vec2<f32>, seed: f32) -> vec2<f32> {
    let cell = uv * cells;
    let row = floor(cell.y);
    let shifted = cell.x + 0.5 * (row - 2.0 * floor(row / 2.0));
    let x = fract(shifted);
    let edge = fract(cell.y + 0.35 * sin(PI * x));
    let size = max(length(fwidth(cell)), 1e-4);
    let fade = 1.0 - smoothstep(0.15, 0.5, size);
    let own = mix(0.93, 1.07, hash1(vec2<f32>(floor(shifted), row), seed));
    // Darker at the root, under the feather before, and a soft dark rim
    // where the tip lies on the next row.
    let root = mix(0.88, 1.0, smoothstep(0.0, 0.6, edge));
    let tip = 1.0 - 0.07 * smoothstep(0.85, 1.0, edge);
    let shade = mix(1.0, own * root * tip, fade);
    let height = SHINGLE_HEIGHT * smoothstep(0.0, 0.9, edge) * (1.0 - smoothstep(0.9, 1.0, edge));
    return vec2<f32>(shade, height * fade);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let uv = in.uv;
    let across = uv.x * 2.0 - 1.0;
    let along = uv.y;
    let shade = in.color.r;
    let code = round(in.color.g * 31.0);
    let kind = floor(code / 8.0 + 0.01);
    let region = code - kind * 8.0;
    let seed = in.color.b * 97.0;
    let wet = plumage.wet;
    let cloth = kind > CLOTH - 0.5;
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
    let cells = vec2<f32>(in.color.b, in.color.r) * CELLS;
    // Derivatives need uniform control flow, so the down is worked out
    // everywhere.
    let down_rows = shingles(uv, cells, 7.0);
    var relief = 0.0;
    if cloth {
        color = plumage.cloth.rgb * knit(uv, cells);
    } else if down && abs(region - FACE) < 0.5 {
        color = face(uv) * down_rows.x;
        relief = down_rows.y;
    } else if down {
        color = region_color(region, 0.0) * DOWN_SHADE * down_rows.x;
        relief = down_rows.y;
    } else {
        // A little darker towards the root, where the next row hides it.
        color *= mix(0.85, 1.0, smoothstep(0.0, 0.5, along));
    }
    // Only plumes show their shafts.
    let shaft = rachis * select(0.0, 1.0, plume);
    color = mix(color, color * 1.2 + 0.02, shaft * 0.3);
    // Water darkens the barbs and leaves a film on them, which only
    // makes them satin, as the barbs stay uneven.
    color *= mix(1.0, 0.6, wet);
    var rough = mix(0.8, 0.55, wet);
    // The fine barbs and the wool fuzz catch light at the rim.
    let rim = pow(1.0 - abs(dot(normalize(in.world_normal), pbr_input.V)), 3.0);
    color *= 1.0 + select(SHEEN, 0.35, cloth) * rim * (1.0 - wet);
    if cloth {
        rough = mix(0.95, 0.6, wet);
    }
    rough = mix(rough, rough * 0.8, shaft);

    pbr_input.material.base_color = vec4<f32>(color, 1.0);
    pbr_input.material.perceptual_roughness = rough;
    pbr_input.material.reflectance = vec3<f32>(mix(0.25, 0.35, wet));
    pbr_input.N = bumped(pbr_input.N, in.world_position.xyz, barb_height + 0.0002 * shaft + relief);

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
