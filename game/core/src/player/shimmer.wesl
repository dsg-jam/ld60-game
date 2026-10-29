import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::view;

struct Shimmer {
    color: vec4<f32>,
    direction: vec2<f32>,
    // From 0 (invisible) to 1 (fully faded in).
    level: f32,
    // Center of the band along `direction`, in screen units.
    band: f32,
    band_width: f32,
}

const BAND_STRENGTH: f32 = 0.5;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> shimmer: Shimmer;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // View space x and y over depth, so the band is straight on screen.
    let view_position = (view.view_from_world * vec4<f32>(in.world_position.xyz, 1.0)).xyz;
    let screen = view_position.xy / -view_position.z;
    let offset = (dot(screen, shimmer.direction) - shimmer.band) / shimmer.band_width;
    let band = exp(-offset * offset);

    let to_eye = normalize(view.world_position - in.world_position.xyz);
    let facing = abs(dot(normalize(in.world_normal), to_eye));
    let rim = pow(1.0 - facing, 3.0);

    // Like a reflection, the band is brighter where the surface turns away.
    let light = BAND_STRENGTH * band * (0.5 + 0.5 * rim);
    let fade = smoothstep(0.0, 1.0, shimmer.level);
    return vec4<f32>(shimmer.color.rgb * light * fade, 0.0);
}
