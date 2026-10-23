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
import ld60_geometry::bump::bumped;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var mask_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var mask_sampler: sampler;

/// Green solder mask over the bare board.
const MASK: vec3<f32> = vec3<f32>(0.04, 0.3, 0.1);
/// Copper seen through the mask: the mask's green over a warm, darker metal.
const MASK_OVER_COPPER: vec3<f32> = vec3<f32>(0.2, 0.5, 0.08);
/// Bare glass fiber board, where a hole goes through.
const LAMINATE: vec3<f32> = vec3<f32>(0.32, 0.29, 0.16);
/// Tinned copper of the open pads.
const SOLDER: vec3<f32> = vec3<f32>(0.85, 0.84, 0.8);
const SILK: vec3<f32> = vec3<f32>(0.95, 0.95, 0.9);
/// The copper stands this far above the board, in meters. Real copper is
/// thinner, but then the bumps would not show from the seat.
const COPPER_HEIGHT: f32 = 0.00008;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let mask = textureSample(mask_texture, mask_sampler, in.uv);
    let copper = mask.r;
    let open = mask.g;
    let silk = mask.b;
    let covered = 1.0 - open;

    let under_mask = mix(MASK, MASK_OVER_COPPER, copper);
    let bare = mix(LAMINATE, SOLDER, copper);
    var color = mix(bare, under_mask, covered);
    let printed = silk * covered;
    color = mix(color, SILK, printed);

    let pad = open * copper;
    pbr_input.material.base_color = vec4<f32>(color, 1.0);
    pbr_input.material.metallic = pad;
    pbr_input.material.perceptual_roughness = mix(mix(0.5, 0.3, pad), 0.8, printed);
    // The mask is a glossy lacquer. Pads, holes and print have none.
    pbr_input.material.clearcoat = covered * (1.0 - printed);
    pbr_input.material.clearcoat_perceptual_roughness = 0.12;

    let normal = bumped(pbr_input.N, in.world_position.xyz, mask.a * COPPER_HEIGHT);
    pbr_input.N = normal;
    pbr_input.clearcoat_N = normal;

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
