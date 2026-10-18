import bevy_pbr::render::{
    mesh_functions::{get_local_from_world, get_world_from_local},
    pbr_fragment::pbr_input_from_standard_material,
};
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
    floor::{plank, plank_color, plank_height, rug_color, rug_height},
    noise::{PI, hash1, noise, visible},
    wear::{dings, floor_marks, grip_ridge, rust, scratches, scuffs},
    wood::{fiber_sheen, wood},
};

struct Surface {
    /// Half the size of a box or cylinder. Zero for freeform meshes and
    /// for scaled unit cubes, which take it from their scale.
    half_size: vec3<f32>,
    seed: f32,
    finish: u32,
    form: u32,
    /// How used the surface looks, from 0 to 1.
    wear: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> surface: Surface;

/// Matches `Finish` in `surface.rs`.
const PAINT: u32 = 0u;
const PLASTIC: u32 = 1u;
const METAL: u32 = 2u;
const WOOD: u32 = 3u;
const PLASTER: u32 = 4u;
const FABRIC: u32 = 5u;
const PAPER: u32 = 6u;
const CERAMIC: u32 = 7u;
const PLANKS: u32 = 8u;
const GRIP: u32 = 9u;
const PERFORATED: u32 = 10u;
const CONCRETE: u32 = 11u;
const RUST: u32 = 12u;
const RUG: u32 = 13u;
/// Matches `Form::code` in `surface.rs`.
const BOX: u32 = 0u;
const CYLINDER: u32 = 1u;
const FREEFORM: u32 = 2u;
const SCALED: u32 = 3u;

const DUST: vec3<f32> = vec3<f32>(0.55, 0.52, 0.47);
const PRIMER: vec3<f32> = vec3<f32>(0.32, 0.31, 0.3);
const RUST_COLOR: vec3<f32> = vec3<f32>(0.3, 0.15, 0.07);
/// Pale plaster under the paint of a wall.
const BARE_PLASTER: vec3<f32> = vec3<f32>(0.62, 0.58, 0.52);
const SCUFF: vec3<f32> = vec3<f32>(0.05, 0.045, 0.04);
const GRIP_DIRT: vec3<f32> = vec3<f32>(0.12, 0.1, 0.08);
/// Ribs around a screwdriver grip.
const GRIP_RIBS: f32 = 10.0;
/// Most light comes from above, which is where the wood's sheen comes from.
const SHEEN_LIGHT: vec3<f32> = vec3<f32>(0.0, 1.0, 0.0);

/// Holes of perforated metal, in staggered rows.
const HOLE_PITCH: f32 = 0.005;
const HOLE_DEPTH: f32 = 0.0015;
const HOLE_RADIUS: f32 = 0.0017;
/// Rows of staggered holes are this much closer than the holes in a row.
const ROW_SPACING: f32 = 0.8660254;
/// Dark cloth shows through the holes.
const CLOTH: f32 = 0.06;

/// How much of the point lies in a hole of a perforated sheet, with the
/// holes in staggered rows. Fades to the share of holes in the sheet where
/// they get smaller than a pixel.
fn hole(uv: vec2<f32>, pixel: f32) -> f32 {
    let row_pitch = HOLE_PITCH * ROW_SPACING;
    let row = floor(uv.y / row_pitch);
    let shifted = vec2<f32>(uv.x / HOLE_PITCH + 0.5 * (row % 2.0), uv.y / row_pitch);
    let local = (fract(shifted) - 0.5) * vec2<f32>(HOLE_PITCH, row_pitch);
    let d = length(local);
    let inside = 1.0 - smoothstep(HOLE_RADIUS - pixel, HOLE_RADIUS + pixel, d);
    let share = PI * HOLE_RADIUS * HOLE_RADIUS / (HOLE_PITCH * row_pitch);
    return mix(share, inside, visible(HOLE_RADIUS * 2.0, pixel));
}

/// Distance from a point on the surface to the nearest edge.
fn edge_distance(p: vec3<f32>, h: vec3<f32>) -> f32 {
    if surface.form == CYLINDER {
        // One of the two is about 0: the face the point lies on.
        return max(h.x - length(p.xz), h.y - abs(p.y));
    }
    // The smallest distance belongs to the face, the middle one to the edge.
    let d = h - abs(p);
    return d.x + d.y + d.z - min(d.x, min(d.y, d.z)) - max(d.x, max(d.y, d.z));
}

/// Coordinates across the face that `n` points out of, with x along the
/// longer side.
fn face_coords(p: vec3<f32>, n: vec3<f32>, h: vec3<f32>) -> vec2<f32> {
    let a = abs(n);
    var uv: vec2<f32>;
    var size: vec2<f32>;
    if a.y > a.x && a.y > a.z {
        uv = p.xz;
        size = h.xz;
    } else if a.x > a.z {
        uv = p.zy;
        size = h.zy;
    } else {
        uv = p.xy;
        size = h.xy;
    }
    return select(uv.yx, uv, size.x >= size.y);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let local = (get_local_from_world(in.instance_index) * in.world_position).xyz;
    let n = normalize(in.world_normal);
    let local_n = normalize((get_local_from_world(in.instance_index) * vec4<f32>(n, 0.0)).xyz);

    var h = surface.half_size;
    var p = local;
    var s = surface.seed;
    if surface.form == SCALED {
        let world_from_local = get_world_from_local(in.instance_index);
        let scale = vec3<f32>(
            length(world_from_local[0].xyz),
            length(world_from_local[1].xyz),
            length(world_from_local[2].xyz),
        );
        h = scale * 0.5;
        p = local * scale;
        // Each box looks a little different.
        s += hash1(floor(world_from_local[3].xz * 3.0), 31.0) * 100.0;
    }
    let shaped = surface.form != FREEFORM;
    var uv = face_coords(p, local_n, h);
    let flooring = surface.finish == PLANKS || surface.finish == RUG;
    if flooring {
        // Floors keep their own directions.
        uv = p.xz;
    }
    let w = surface.wear;
    // Size of a pixel on the surface. Detail finer than that fades out.
    let pixel = length(fwidth(p));
    var color = pbr_input.material.base_color.rgb;
    var rough = pbr_input.material.perceptual_roughness;
    var metallic = pbr_input.material.metallic;
    var bumps = 0.0;
    // Handled things get scratched.
    var scratchable = false;

    // Detail of each kind of surface.
    switch surface.finish {
        case PAINT: {
            // Orange peel, barely there.
            bumps = 0.00002 * noise(uv * 120.0, s + 4.0) * visible(0.008, pixel);
            scratchable = true;
        }
        case PLASTIC: {
            // Large, faint differences in color and sheen.
            color *= mix(0.97, 1.02, noise(uv * 3.0, s));
            rough = mix(rough, rough + 0.08, noise(uv * 2.0, s + 1.0));
            bumps = 0.00002 * noise(uv * 120.0, s + 4.0) * visible(0.008, pixel);
            scratchable = true;
        }
        case METAL: {
            let brushed = noise(vec2<f32>(uv.x * 3.0, uv.y * 400.0), s) * visible(0.0025, pixel);
            rough = mix(rough * 0.7, rough * 1.2, brushed);
            // Fingerprints dull the shine in patches.
            rough = mix(rough, rough + 0.2, smoothstep(0.6, 0.8, noise(uv * 30.0, s + 2.0)));
            let rusty = rust(uv, s, w, pixel);
            color = mix(color, RUST_COLOR * mix(0.7, 1.2, noise(uv * 300.0, s + 3.0)), rusty);
            rough = mix(rough, 0.9, rusty);
            metallic = mix(metallic, 0.0, rusty);
            bumps = 0.00005 * noise(uv * 400.0, s + 5.0) * visible(0.0025, pixel) - 0.0002 * rusty;
            scratchable = true;
        }
        case WOOD: {
            let grain = wood(p, h, s, pixel);
            let fiber = normalize((get_world_from_local(in.instance_index) * vec4<f32>(grain.fiber, 0.0)).xyz);
            let sheen = fiber_sheen(fiber, pbr_input.V, SHEEN_LIGHT);
            color *= grain.shade * (1.0 + 0.3 * sheen);
            rough *= grain.roughness;
            pbr_input.material.reflectance = vec3<f32>(0.35);
            bumps = grain.height;
            scratchable = true;
        }
        case PLASTER: {
            color *= mix(0.8, 1.1, noise(uv * 2.0, s)) * mix(0.9, 1.05, noise(uv * 25.0, s + 1.0));
            pbr_input.material.reflectance = vec3<f32>(0.25);
            let pores = step(0.96, hash1(floor(uv * 300.0), s)) * visible(0.0033, pixel);
            // Chips knocked out of the plaster, and shoe marks low down.
            let ding = dings(uv, s, w, pixel);
            color = mix(color, BARE_PLASTER, ding.hole * 0.7) * (1.0 - 0.4 * ding.rim);
            color = mix(color, SCUFF, scuffs(uv, p.y + h.y, s, w) * 0.5);
            bumps = -0.0004 * pores + 0.0006 * noise(uv * 40.0, s + 1.0) - 0.0015 * ding.hole;
        }
        case FABRIC: {
            color *= mix(0.9, 1.05, noise(uv * 15.0, s));
            pbr_input.material.reflectance = vec3<f32>(0.2);
            bumps = 0.0002 * sin(uv.x * 900.0) * sin(uv.y * 900.0) * visible(0.007, pixel);
        }
        case PAPER: {
            color *= mix(0.94, 1.02, noise(uv * 40.0, s));
        }
        case CERAMIC: {
            // Glaze pooled a little thicker in places.
            color *= mix(0.97, 1.01, noise(uv * 8.0, s));
            rough = mix(rough, rough * 1.4, noise(uv * 5.0, s + 1.0));
        }
        case PLANKS: {
            let board = plank(uv, s);
            let marks = floor_marks(uv, s, w, pixel);
            color = plank_color(color, board) * (1.0 - 0.3 * marks);
            rough = mix(rough, 0.85, marks);
            bumps = plank_height(board) - 0.0004 * marks;
        }
        case GRIP: {
            // Dirt settles between the ribs, the ribs wear shiny.
            let ridge = grip_ridge(p, GRIP_RIBS);
            let dirt = (1.0 - ridge) * w * mix(0.5, 1.0, noise(vec2<f32>(p.y * 60.0, ridge), s + 6.0));
            color = mix(color, GRIP_DIRT, dirt * 0.6);
            rough = mix(rough * 1.25, rough * 0.6, ridge * w);
            bumps = 0.0005 * ridge;
        }
        case CONCRETE: {
            color *= mix(0.8, 1.1, noise(uv * 2.0, s)) * mix(0.9, 1.05, noise(uv * 25.0, s + 1.0));
            pbr_input.material.reflectance = vec3<f32>(0.25);
            let pores = step(0.93, hash1(floor(uv * 300.0), s)) * visible(0.0033, pixel);
            bumps = -0.0006 * pores + 0.0006 * noise(uv * 40.0, s + 1.0);
        }
        case RUST: {
            let patches = smoothstep(0.4, 0.75, noise(uv * 6.0, s) * 0.7 + noise(uv * 30.0, s + 1.0) * 0.3);
            color = mix(color, RUST_COLOR * mix(0.7, 1.2, noise(uv * 50.0, s + 2.0)), patches);
            metallic *= 1.0 - patches;
            bumps = (0.0006 * noise(uv * 60.0, s + 2.0) + 0.0003 * noise(uv * 200.0, s + 3.0)) * visible(0.005, pixel);
        }
        case RUG: {
            color = rug_color(uv, h.xz, color, s);
            pbr_input.material.reflectance = vec3<f32>(0.2);
            bumps = rug_height(uv, pixel);
        }
        case PERFORATED: {
            let open = hole(uv, pixel);
            color = mix(color, color * CLOTH, open);
            rough = mix(rough, 1.0, open);
            metallic *= 1.0 - open;
            bumps = -HOLE_DEPTH * open;
        }
        default: {}
    }

    // Edges wear lighter. Paint chips down to primer.
    let smallest = min(h.x, min(h.y, h.z));
    let width = clamp(smallest * 0.12, 0.002, 0.02) * mix(0.3, 1.6, noise(uv * 40.0, s + 6.0));
    let edge = (1.0 - smoothstep(0.0, width, edge_distance(p, h))) * w * f32(shaped);
    if surface.finish == PAINT {
        let chipped = edge * step(0.45, noise(uv * 25.0, s + 7.0));
        color = mix(mix(color, color * 1.3 + 0.05, edge), PRIMER, chipped);
        rough = mix(rough, 0.6, chipped);
    } else if surface.finish != FABRIC && surface.finish != PAPER && surface.finish != RUG {
        color = mix(color, color * 1.3 + 0.05, edge * 0.7);
        rough = mix(rough, rough * 0.7, edge);
    }

    // Grime near the bottom and in patches.
    let low = (1.0 - smoothstep(0.0, h.y * 1.2, p.y + h.y)) * f32(shaped);
    let patches = smoothstep(0.6, 0.95, noise(p.xz * 3.0 + vec2<f32>(p.y * 2.0), s + 8.0));
    // Patches on a floor look like a blurry stain.
    let grime = w * clamp(low * 0.5 + patches * 0.4 * f32(!flooring), 0.0, 1.0);
    color *= 1.0 - 0.35 * grime;
    rough = mix(rough, min(rough + 0.2, 1.0), grime);

    // A thin film of dust on faces that look up.
    let up = smoothstep(0.6, 1.0, n.y);
    let film = 0.6 * noise(p.xz * 4.0, s + 9.0) + 0.4 * noise(p.xz * 12.0, s + 10.0);
    let dust = w * up * smoothstep(0.2, 0.9, film) * f32(!flooring);
    color = mix(color, DUST, dust * 0.3);
    rough = mix(rough, 1.0, dust * 0.5);

    let scratched = scratches(uv, s, w, pixel) * f32(scratchable);
    color = mix(color, color * 1.25 + 0.04, scratched);
    rough = mix(rough, rough * 0.6, scratched);

    pbr_input.material.base_color = vec4<f32>(color, pbr_input.material.base_color.a);
    pbr_input.material.perceptual_roughness = clamp(rough, 0.04, 1.0);
    pbr_input.material.metallic = metallic;
    pbr_input.N = bumped(pbr_input.N, in.world_position.xyz, bumps);

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
