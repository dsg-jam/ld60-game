/// Bare and plated steel. The detail lives in the roughness: brushed grain,
/// smudges, fine scratches along the way it was handled and polished
/// edges. The color stays clean, as metal gets its color from what it
/// reflects. Bare steel rusts from its edges and scratches.
///
/// `pixel` is the size of a pixel on the surface in meters. Detail finer
/// than that fades to its average, so highlights don't sparkle.

import package::geometry::noise::{noise, specks, visible};

/// Bare steel showing where plating wore through.
const BARE_STEEL: vec3<f32> = vec3<f32>(0.53, 0.51, 0.49);
/// Iron oxide, from the dark brown of old rust to the orange of fresh rust.
const OLD_RUST: vec3<f32> = vec3<f32>(0.1, 0.04, 0.018);
const FRESH_RUST: vec3<f32> = vec3<f32>(0.3, 0.11, 0.035);
/// How far rust stands up from the steel, in meters.
const RUST_HEIGHT: f32 = 0.00015;

struct Metal {
    color: vec3<f32>,
    roughness: f32,
    metallic: f32,
    /// In meters, for bump mapping.
    height: f32,
}

/// Smooth noise in two octaves, turned against each other so the grid of
/// the noise doesn't show.
fn fbm(p: vec2<f32>, s: f32) -> f32 {
    let turned = mat2x2<f32>(0.8, -0.6, 0.6, 0.8) * p * 2.03;
    return noise(p, s) * 0.65 + noise(turned, s + 1.0) * 0.35;
}

/// Lines along x from use and the finish, with `width` meters across.
/// Returns about 0.5 where they get too fine to see.
fn grain(uv: vec2<f32>, width: f32, s: f32, pixel: f32) -> f32 {
    let lines = noise(vec2<f32>(uv.x / (width * 120.0), uv.y / width), s);
    return mix(0.5, lines, visible(width, pixel));
}

/// Steel of `base` color and `roughness` at `uv`, with x along the way it
/// was brushed and handled. `edge` is 1 on a worn edge, `damp` near its
/// bottom where water collects. `scratched` is how scratched the point is.
/// Plated steel shows bare steel where it wore through, bare steel rusts.
fn steel(
    base: vec3<f32>,
    roughness: f32,
    uv: vec2<f32>,
    edge: f32,
    damp: f32,
    scratched: f32,
    plated: bool,
    s: f32,
    wear: f32,
    pixel: f32,
) -> Metal {
    var out: Metal;
    // Brushed grain in two sizes, and wide smudges of handling.
    let coarse = grain(uv, 0.0006, s + 50.0, pixel);
    let fine = grain(uv, 0.00015, s + 51.0, pixel);
    let smudge = fbm(uv * 35.0, s + 52.0);
    out.roughness = roughness * (0.85 + 0.18 * coarse + 0.12 * fine) + 0.08 * (smudge - 0.5);
    out.color = base * (0.98 + 0.04 * coarse);
    out.metallic = 1.0;
    // Long dents from stamping and knocks, so reflections waver a little.
    out.height = 0.00002 * fbm(uv * 25.0, s + 53.0);

    // Edges get polished by hands. Plating wears through to the steel.
    // Plating chips off in small flakes along the edges.
    let flakes = edge * fbm(uv * 900.0, s + 54.0) * (0.4 + 0.6 * noise(uv * 80.0, s + 60.0)) * visible(0.001, pixel);
    let blur = max(0.03, fwidth(flakes));
    let worn = smoothstep(0.45 - blur, 0.45 + blur, flakes);
    if plated {
        out.color = mix(out.color, BARE_STEEL, worn * 0.7);
    }
    out.roughness = mix(out.roughness, out.roughness * 0.6, max(worn, edge * 0.5));
    out.roughness = mix(out.roughness, out.roughness * 0.55, scratched);
    out.color *= 1.0 + 0.08 * scratched;

    // Rust gathers in clusters, starting at edges and in scratches.
    let small = mix(0.5, fbm(uv * 160.0, s + 56.0), visible(0.006, pixel));
    let field = fbm(uv * 40.0, s + 55.0) * 0.75 + small * 0.25
        + 0.12 * edge + 0.25 * scratched + 0.12 * damp;
    let rim = max(0.02, fwidth(field));
    if !plated {
        let level = 1.0 - 0.32 * wear;
        let rust = smoothstep(level - rim, level + rim, field);
        // Pits around it where it started.
        let pits = specks(uv, 0.002, 0.25, 0.00012, 0.00025, s + 57.0, pixel)
            * smoothstep(level - 0.08, level, field);
        let crust = mix(0.5, noise(uv * 2500.0, s + 58.0), visible(0.0004, pixel));
        let tone = fbm(uv * 400.0, s + 59.0);
        let covered = max(rust, pits * 0.8);
        out.color = mix(out.color, mix(OLD_RUST, FRESH_RUST, tone) * (0.8 + 0.4 * crust), covered);
        out.roughness = mix(out.roughness, 0.8 + 0.15 * crust, covered);
        out.metallic = 1.0 - covered;
        out.height += RUST_HEIGHT * rust * (0.6 + 0.4 * crust) - 0.00005 * pits;
    }
    return out;
}
