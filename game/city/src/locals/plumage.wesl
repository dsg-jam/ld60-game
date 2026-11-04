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
    /// The color of tips, bars and spots.
    accent: vec4<f32>,
    /// Matches `Pattern` in `plumage.rs`.
    pattern: u32,
    /// How soaked the feathers are, from 0 to 1.
    wet: f32,
    /// How strongly the iridescent feathers shift in color.
    sheen: f32,
    seed: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> plumage: Plumage;

/// Matches `Pattern` in `plumage.rs`.
const PLAIN: u32 = 0u;
const BARRED: u32 = 1u;
const TIPPED: u32 = 2u;
const SPOTTED: u32 = 3u;
/// Matches `Kind` in `plumage/feathers.rs`, as the green of the vertex color
/// times 2.
const CONTOUR: f32 = 0.0;
const SHIMMER: f32 = 1.0;
const PLUME: f32 = 2.0;
const DOWN: f32 = 3.0;
/// Down is darker than the feathers over it.
const DOWN_SHADE: f32 = 0.45;
/// Barbs across the vane of a feather.
const BARBS: f32 = 70.0;
/// Barbs sweep towards the tip.
const SWEEP: f32 = 0.7;

/// Half the width of a vane at `along` from the root at 0 to the tip at 1,
/// as a share of the card. Contour feathers are broad and round, plumes
/// narrow with a wider tip.
fn vane(along: f32, kind: f32) -> f32 {
    let t = clamp(along, 0.0, 1.0);
    let contour = sqrt(max(sin(PI * min(t * 0.85 + 0.12, 1.0)), 0.0));
    let plume = mix(0.25, 0.9, smoothstep(0.3, 0.95, t)) * sqrt(max(1.0 - t, 0.0)) * 1.6;
    return mix(contour, plume, step(1.5, kind));
}

/// A color that shifts with the angle it is seen at, like the neck of a
/// pigeon: thin layers in the barbs reflect a hue that depends on the path
/// of the light through them.
fn shimmer(facing: f32, base: vec3<f32>) -> vec3<f32> {
    let phase = 2.0 * PI * (1.0 - facing) * 1.3 + plumage.seed;
    let hue = 0.5 + 0.5 * cos(vec3<f32>(phase, phase + 2.1, phase + 4.2));
    return mix(base, hue * 0.35 + base * 0.4, plumage.sheen);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let uv = in.uv;
    let across = uv.x * 2.0 - 1.0;
    let along = uv.y;
    let shade = in.color.r;
    let kind = round(in.color.g * 2.0);
    let seed = in.color.b * 97.0;
    let wet = plumage.wet;

    // Wet barbs clump together into a few strands, with gaps between.
    let gaps = mix(0.0, 0.35, wet) + 0.12 * step(0.75, hash1(vec2<f32>(seed, 3.0), 2.0));
    let notch = noise(vec2<f32>(along * 9.0, seed), 4.0);
    let width = vane(along, kind) * (1.0 - gaps * step(0.55, notch) * step(0.4, abs(across)));
    let down = kind > DOWN - 0.5;
    if !down && (abs(across) > width || along > 0.995) {
        discard;
    }

    let pixel = length(fwidth(in.world_position.xyz));
    let barb_count = BARBS * mix(1.0, 0.35, wet);
    let barb = fract((along + abs(across) * SWEEP) * barb_count);
    let barb_height = 0.00012 * (1.0 - abs(barb * 2.0 - 1.0)) * visible(0.003, pixel);
    let rachis = 1.0 - smoothstep(0.03, 0.07, abs(across));

    var color = pbr_input.material.base_color.rgb * shade;
    // Darker towards the root, where the down hides under the next row.
    color *= mix(0.55, 1.0, smoothstep(0.0, 0.5, along));
    // The edge of the vane catches less dye.
    color *= mix(1.0, 0.85, smoothstep(0.6, 1.0, abs(across) / max(width, 0.01)));
    let accent = plumage.accent.rgb;
    switch plumage.pattern {
        case BARRED: {
            let bar = smoothstep(0.55, 0.8, abs(fract(along * 2.5 + 0.3) - 0.5) * 2.0);
            color = mix(color, accent, bar * 0.6 * smoothstep(0.3, 0.6, along));
        }
        case TIPPED: {
            color = mix(color, accent, smoothstep(0.72, 0.82, along));
        }
        case SPOTTED: {
            let spot = 1.0 - smoothstep(0.1, 0.16, length(vec2<f32>(across * 0.5, along - 0.78)));
            color = mix(color, accent, spot);
        }
        default: {}
    }
    if kind > 1.5 {
        // Plumes have their own color at the tip.
        color = mix(color, accent, smoothstep(0.55, 0.9, along));
    }
    if down {
        color = pbr_input.material.base_color.rgb * DOWN_SHADE;
    }
    if abs(kind - SHIMMER) < 0.5 {
        color = shimmer(abs(dot(normalize(in.world_normal), pbr_input.V)), color);
    }
    // Only plumes show their shafts.
    let shaft = rachis * step(1.5, kind) * step(kind, 2.5);
    color = mix(color, color * 1.4 + 0.03, shaft * 0.3);
    // Water darkens the barbs and leaves a film on them.
    color *= mix(1.0, 0.55, wet);
    var rough = mix(0.75, 0.35, wet);
    rough = mix(rough, rough * 0.8, shaft);

    pbr_input.material.base_color = vec4<f32>(color, 1.0);
    pbr_input.material.perceptual_roughness = rough;
    pbr_input.material.reflectance = vec3<f32>(mix(0.3, 0.5, wet));
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
