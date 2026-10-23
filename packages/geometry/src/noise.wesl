/// Hashes, smooth noise and fading detail, shared by the shaders.

/// The PCG hash, from Jarzynski and Olano, "Hash Functions for GPU
/// Rendering".
fn pcg(seed: u32) -> u32 {
    let state = seed * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

const PI: f32 = 3.1415927;
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

/// Specks scattered over a grid of `cell` meters, in about `density` of the
/// cells. Each is an ellipse at a random angle, with radii from `small` to
/// `large`, so some are round and some stretched like fibers. Returns how much
/// of the pixel a speck covers.
fn specks(p: vec2<f32>, cell: f32, density: f32, small: f32, large: f32, s: f32, pixel: f32) -> f32 {
    let base = floor(p / cell);
    var cover = 0.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let c = base + vec2<f32>(f32(i), f32(j));
            let h = hash(c, s);
            if h.x > density {
                continue;
            }
            let angle = h.w * TAU;
            let along = vec2<f32>(cos(angle), sin(angle));
            let d = p - (c + h.yz) * cell;
            let radii = vec2<f32>(mix(small, large, fract(h.w * 37.0)), small);
            // Thinner than a pixel, a speck spreads its color over the pixel.
            let drawn = max(radii, vec2<f32>(pixel * 0.5));
            let q = vec2<f32>(dot(d, along), dot(d, vec2<f32>(-along.y, along.x))) / drawn;
            let r = length(q);
            let distance = (r - 1.0) * r / max(length(q / drawn), 1e-6);
            let faint = radii.x * radii.y / (drawn.x * drawn.y);
            cover = max(cover, clamp(0.5 - distance / pixel, 0.0, 1.0) * faint);
        }
    }
    return cover;
}
