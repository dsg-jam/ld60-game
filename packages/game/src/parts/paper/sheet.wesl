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
import package::geometry::{
    bump::bumped,
    noise::{TAU, hash, noise, visible},
};
import package::parts::paper::punch::punched;
import package::parts::paper::press::printed;
import package::parts::paper::stock::toned;

struct Sheet {
    size: vec2<f32>,
    seed: f32,
    page: f32,
    /// How much grime and stains show, from 0 to 1.
    stains: f32,
    /// How the page was printed, see `press.wesl`.
    press: u32,
    run: f32,
    /// The paper's tint, and in `w` its age, see `stock.wesl`.
    stock: vec4<f32>,
    /// Point on the fold, its angle and depth, in meters from the top left.
    folds: array<vec4<f32>, 2>,
    /// Center and radius of each punched hole, in meters from the top left.
    holes: array<vec4<f32>, 2>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> sheet: Sheet;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var print_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var print_sampler: sampler;

const COFFEE: vec3<f32> = vec3<f32>(0.78, 0.6, 0.4);
const GRIME: vec3<f32> = vec3<f32>(0.7, 0.64, 0.54);
/// Spacing of the ridges of a fingerprint, in meters.
const RIDGES: f32 = 0.00045;

/// Distance of `p` from a fold line.
fn fold_distance(p: vec2<f32>, fold: vec4<f32>) -> f32 {
    let normal = vec2<f32>(-sin(fold.z), cos(fold.z));
    return abs(dot(p - fold.xy, normal));
}

/// How much grease a thumb left around `center`, from 0 to 1.
fn fingerprint(p: vec2<f32>, center: vec2<f32>, angle: f32, pixel: f32, salt: f32) -> f32 {
    let c = cos(angle);
    let s = sin(angle);
    let q = mat2x2<f32>(c, -s, s, c) * (p - center);
    let shape = length(q / vec2<f32>(0.0075, 0.0095));
    let mask = 1.0 - smoothstep(0.5, 1.0, shape + 0.3 * noise(q * 700.0, salt));
    let r = length(q * vec2<f32>(1.0, 0.8)) + 0.12 * q.y + 0.0008 * noise(q * 350.0, salt + 1.0);
    let ridges = 0.5 + 0.5 * sin(r * TAU / RIDGES);
    let shown = visible(RIDGES, pixel);
    return mask * mix(0.5, ridges, shown);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let size = sheet.size;
    let s = sheet.seed + sheet.page * 7.0;
    // Meters from the top left corner.
    let p = in.uv * size;
    let pixel = max(length(dpdx(p)), length(dpdy(p)));

    // Sharp fold lines and paper fibers. The mesh has the broad shape.
    var fold_line = 0.0;
    var relief = 0.0;
    for (var i = 0; i < 2; i++) {
        let fold = sheet.folds[i];
        let d = fold_distance(p, fold);
        relief += fold.w * 0.3 * exp(-d / 0.0015);
        fold_line += exp(-d / 0.0006) * step(0.00001, abs(fold.w));
    }
    let fiber = noise(p * vec2<f32>(2500.0, 900.0), sheet.seed);
    relief += 0.000015 * fiber * visible(0.0004, pixel);
    var left = 1.0;
    var crease = 0.0;
    for (var i = 0; i < 2; i++) {
        let punch = punched(p, size, sheet.holes[i].xyz, s, pixel);
        left = min(left, punch.x);
        crease = max(crease, punch.y);
    }
    relief += 0.0002 * crease * (fiber - 0.5);
    let printing = printed(print_texture, print_sampler, in.uv, dpdx(in.uv), dpdy(in.uv), sheet.press, sheet.run, s);
    relief += printing.relief;
    pbr_input.N = bumped(pbr_input.N, in.world_position.xyz, relief);

    let ink = printing.color;
    // 1 on bare paper, 0 under the ink, so stains stay under the print.
    let bare = smoothstep(0.25, 0.75, dot(ink, vec3<f32>(0.3, 0.6, 0.1)));
    var stain = vec3<f32>(1.0);

    // Grain and uneven pulp.
    stain *= 0.975 + 0.04 * noise(p * 3500.0, sheet.seed + 1.0) * visible(0.0003, pixel)
        + 0.02 * fiber * visible(0.0004, pixel);
    stain *= toned(p, size, sheet.stock, sheet.run, s, pixel);

    // Grime: blotches, and dirt along the edges where hands hold it.
    let blotch = noise(p * 20.0, s + 3.0) * 0.6 + noise(p * 70.0, s + 4.0) * 0.4;
    var grime = 0.35 * smoothstep(0.45, 0.9, blotch);
    let edge = min(min(p.x, size.x - p.x), min(p.y, size.y - p.y));
    let reach = 0.002 + 0.005 * noise(vec2<f32>(p.x + p.y, 0.5) * 120.0, s + 5.0);
    grime += 0.6 * (1.0 - smoothstep(0.0, reach, edge));
    grime += 0.25 * fold_line + 0.4 * crease;

    // Thumbs along the bottom and the sides.
    for (var i = 0; i < 3; i++) {
        let h = hash(vec2<f32>(s, f32(i)), 70.0);
        if h.w > 0.55 {
            continue;
        }
        let side = floor(h.z * 3.0);
        var center = vec2<f32>(mix(0.15, 0.85, h.x) * size.x, size.y - 0.008);
        if side == 1.0 {
            center = vec2<f32>(0.008, mix(0.4, 0.95, h.y) * size.y);
        } else if side == 2.0 {
            center = vec2<f32>(size.x - 0.008, mix(0.4, 0.95, h.y) * size.y);
        }
        let thumb = fingerprint(p, center, h.y * TAU, pixel, s + 10.0 + f32(i));
        grime += 0.6 * thumb;
        pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, 0.7, thumb * sheet.stains);
    }
    stain *= mix(vec3<f32>(1.0), GRIME, min(grime, 1.0) * sheet.stains);

    // A ring left by the bottom of a mug, often partly off the page.
    let h = hash(vec2<f32>(s, 3.0), 80.0);
    if h.w < 0.5 * sheet.stains {
        let center = size * (vec2<f32>(h.x, h.y) * 1.3 - 0.15);
        let radius = mix(0.034, 0.042, h.z);
        let r = length(p - center) + 0.0012 * noise(p * 150.0, s + 6.0);
        let rim = exp(-pow((r - radius) / 0.0009, 2.0));
        let inside = (1.0 - smoothstep(radius - 0.003, radius, r)) * 0.25;
        let patchy = 0.3 + 0.7 * noise(p * 45.0, s + 7.0);
        let coffee = clamp(rim * 0.6 + inside, 0.0, 1.0) * patchy;
        stain *= mix(vec3<f32>(1.0), COFFEE, coffee * 0.55);
    }

    let paper = ink * mix(vec3<f32>(1.0), stain, bare);
    pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * paper, 1.0);
    pbr_input.material.emissive = vec4<f32>(pbr_input.material.emissive.rgb * paper, pbr_input.material.emissive.a);

    if left < 0.5 {
        discard;
    }
    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
