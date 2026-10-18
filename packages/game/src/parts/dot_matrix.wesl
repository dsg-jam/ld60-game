import bevy_pbr::render::{forward_io::VertexOutput, mesh_view_bindings::view};

/// Matches `DotMatrixParams` in `dot_matrix.rs`.
struct DotMatrix {
    /// 1 while the backlight is on.
    lit: f32,
    /// Visible columns.
    columns: u32,
    /// First column of the message on the left edge.
    offset: u32,
    /// Columns in the message before it repeats.
    length: u32,
    /// How far behind the dots their shadows fall, in dots.
    shadow_depth: f32,
    /// One byte per column, the top row in bit 0.
    text: array<vec4<u32>, 32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> screen: DotMatrix;

/// Matches `ROWS` and `MARGIN` in `dot_matrix.rs`.
const ROWS: f32 = 7.0;
const MARGIN: f32 = 1.0;
/// Half the width of a dot, as a share of its cell.
const DOT_HALF: f32 = 0.4;
/// How dark dots are when on, and when off but faintly showing.
const INK_ON: f32 = 0.92;
const INK_OFF: f32 = 0.06;
const BACKLIGHT: vec3<f32> = vec3<f32>(0.5, 0.7, 0.16);
const DARK: vec3<f32> = vec3<f32>(0.035, 0.045, 0.025);
const INK: vec3<f32> = vec3<f32>(0.03, 0.05, 0.01);
/// Seen nearly edge on, the shift would grow without bound.
const MIN_FACING: f32 = 0.2;
/// How much light the shadow of a dark dot takes away.
const SHADOW: f32 = 0.35;

fn dot_on(column: u32, row: u32) -> bool {
    let m = (column + screen.offset) % max(screen.length, 1u);
    let word = screen.text[m / 16u][(m / 4u) % 4u];
    let byte = (word >> ((m % 4u) * 8u)) & 0xffu;
    return ((byte >> row) & 1u) != 0u;
}

/// How dark the point `grid` is, in dots from the top left.
fn ink(grid: vec2<f32>, pixel: f32) -> f32 {
    let columns = f32(screen.columns);
    if any(grid < vec2<f32>(0.0)) || any(grid >= vec2<f32>(columns, ROWS)) {
        return 0.0;
    }
    let cell = vec2<u32>(grid);
    let f = fract(grid) - 0.5;
    let in_dot = (1.0 - smoothstep(DOT_HALF - pixel, DOT_HALF + pixel, abs(f.x)))
        * (1.0 - smoothstep(DOT_HALF - pixel, DOT_HALF + pixel, abs(f.y)));
    return select(INK_OFF, INK_ON, dot_on(cell.x, cell.y)) * in_dot;
}

/// Where the shadows fall, in dots, seen from the camera. The shadow of a
/// dot lies behind it, so it shows shifted towards the camera.
fn shadow_shift(in: VertexOutput, grid: vec2<f32>) -> vec2<f32> {
    // How the grid runs across the surface, in world space, from how both
    // change between pixels. The dots are square, so one scale fits both
    // directions.
    let p = in.world_position.xyz;
    let n = normalize(in.world_normal);
    let dp1 = dpdx(p);
    let dp2 = dpdy(p);
    let dg1 = dpdx(grid);
    let dg2 = dpdy(grid);
    let across = cross(dp2, n) * dg1.x + cross(n, dp1) * dg2.x;
    let down = cross(dp2, n) * dg1.y + cross(n, dp1) * dg2.y;
    let scale = inverseSqrt(max(dot(across, across), dot(down, down)));
    let to_eye = normalize(view.world_position - p);
    let facing = max(dot(to_eye, n), MIN_FACING);
    return vec2<f32>(dot(to_eye, across), dot(to_eye, down)) * scale / facing * screen.shadow_depth;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let columns = f32(screen.columns);
    let grid = in.uv * vec2<f32>(columns, ROWS) + (in.uv * 2.0 - 1.0) * MARGIN;
    // Derivatives need uniform control flow, so take them up front.
    let pixel = fwidth(grid.x);
    let shift = shadow_shift(in, grid);
    let backlight = mix(DARK, BACKLIGHT, screen.lit);
    let shadow = ink(grid + shift, pixel) * SHADOW * screen.lit;
    let dark = ink(grid, pixel) * screen.lit;
    return vec4<f32>(mix(backlight * (1.0 - shadow), INK, dark), 1.0);
}
