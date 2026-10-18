/// Hashes, smooth noise and fading detail, shared by the shaders.

/// The PCG hash, from Jarzynski and Olano, "Hash Functions for GPU
/// Rendering".
fn pcg(seed: u32) -> u32 {
    let state = seed * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

const TAU: f32 = 6.2831853;
const MAX_U32: f32 = 4294967295.0;

/// Random bits for a cell of a grid, different for each `salt`.
fn cell_state(cell: vec2<f32>, salt: f32) -> u32 {
    return pcg(bitcast<u32>(i32(cell.x)) ^ pcg(bitcast<u32>(i32(cell.y)) ^ pcg(u32(salt * 977.0))));
}

/// Four random numbers from 0 to 1 for a cell.
fn hash(cell: vec2<f32>, salt: f32) -> vec4<f32> {
    var state = cell_state(cell, salt);
    var out: vec4<f32>;
    for (var i = 0; i < 4; i++) {
        state = pcg(state);
        out[i] = f32(state) / MAX_U32;
    }
    return out;
}

/// One random number from 0 to 1 for a cell.
fn hash1(cell: vec2<f32>, salt: f32) -> f32 {
    return f32(cell_state(cell, salt)) / MAX_U32;
}

/// Smooth noise from 0 to 1 with features of size 1.
fn noise(p: vec2<f32>, salt: f32) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash1(i, salt);
    let b = hash1(i + vec2<f32>(1.0, 0.0), salt);
    let c = hash1(i + vec2<f32>(0.0, 1.0), salt);
    let d = hash1(i + vec2<f32>(1.0, 1.0), salt);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// How much of a feature `size` meters wide still shows where a pixel
/// covers `pixel` meters. Finer detail fades out, so it doesn't sparkle.
fn visible(size: f32, pixel: f32) -> f32 {
    return 1.0 - smoothstep(0.5, 1.5, pixel / size);
}
