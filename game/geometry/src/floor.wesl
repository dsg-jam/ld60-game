import package::noise::{TAU, hash1, noise, specks, visible};

const PLANK_WIDTH: f32 = 0.14;
const PLANK_LENGTH: f32 = 1.1;

/// A point on a floor of boards running along y.
struct Plank {
    /// Random per board, from 0 to 1.
    id: f32,
    /// 1 on a board, 0 in the gap between two.
    on_board: f32,
    /// Grain lines across the board, from 0 to 1.
    grain: f32,
}

fn plank(uv: vec2<f32>, s: f32) -> Plank {
    let row = floor(uv.x / PLANK_WIDTH);
    let along = uv.y + hash1(vec2<f32>(row, 0.0), s + 40.0) * PLANK_LENGTH;
    let column = floor(along / PLANK_LENGTH);
    let across = uv.x - row * PLANK_WIDTH;
    let inside = along - column * PLANK_LENGTH;
    var out: Plank;
    out.id = hash1(vec2<f32>(row, column), s + 41.0);
    let seam = min(min(across, PLANK_WIDTH - across), min(inside, PLANK_LENGTH - inside));
    out.on_board = smoothstep(0.001, 0.003, seam);
    let wobble = noise(vec2<f32>(across * 20.0, along * 1.5), s + out.id * 10.0);
    out.grain = 0.5 + 0.5 * sin(across * 260.0 + wobble * 6.0);
    return out;
}

fn plank_color(color: vec3<f32>, board: Plank) -> vec3<f32> {
    return color * mix(0.7, 1.15, board.id) * mix(0.85, 1.05, board.grain) * mix(0.3, 1.0, board.on_board);
}

fn plank_height(board: Plank) -> f32 {
    return -0.0008 * (1.0 - board.on_board) + 0.0003 * board.grain;
}

const KNOT: f32 = 0.01;
const SELVEDGE: f32 = 0.025;
const BORDER: f32 = 0.13;
const LINE: f32 = 0.02;
const MOTIF: f32 = 0.1;

const RUG_DARK: vec3<f32> = vec3<f32>(0.07, 0.045, 0.04);
const RUG_CREAM: vec3<f32> = vec3<f32>(0.62, 0.54, 0.4);
const RUG_INDIGO: vec3<f32> = vec3<f32>(0.08, 0.11, 0.24);
const RUG_OCHRE: vec3<f32> = vec3<f32>(0.5, 0.33, 0.1);
const RUG_WARP: vec3<f32> = vec3<f32>(0.45, 0.4, 0.33);
const RUG_DIRT: vec3<f32> = vec3<f32>(0.1, 0.075, 0.055);
const RUG_LINT: vec3<f32> = vec3<f32>(0.6, 0.58, 0.55);
const COFFEE: vec3<f32> = vec3<f32>(0.55, 0.38, 0.22);

/// A point on the rug.
struct Rug {
    color: vec3<f32>,
    /// How flat the pile is worn, from 0 to 1.
    worn: f32,
    /// How much grease makes it shine, from 0 to 1.
    gloss: f32,
}

/// A knotted rug with a border and a medallion, lying flat with its length
/// along y and `field` as the main color. `h` is its half size. The end at
/// -y lies under the counter, where the seated player keeps their feet.
fn rug(p: vec2<f32>, h: vec2<f32>, field: vec3<f32>, s: f32, pixel: f32) -> Rug {
    // The pattern is made of knots, which gives it stepped outlines.
    let q = (floor(p / KNOT) + 0.5) * KNOT;
    let to_side = h.x - abs(q.x);
    let to_end = h.y - abs(q.y);
    let edge = min(to_side, to_end);

    var color = field;
    if edge < SELVEDGE {
        color = RUG_DARK;
    } else if edge < BORDER {
        // Diamonds running around the border.
        let along = select(q.x, q.y, to_side < to_end);
        let offset = abs(edge - (SELVEDGE + BORDER) / 2.0);
        let diamond = abs(fract(along / MOTIF) - 0.5) * MOTIF + offset;
        color = RUG_INDIGO;
        if diamond < 0.04 {
            color = RUG_CREAM;
        }
        if diamond < 0.025 {
            color = RUG_OCHRE;
        }
        if diamond < 0.01 {
            color = field;
        }
    } else if edge < BORDER + LINE {
        color = RUG_CREAM;
    } else {
        let field_size = h - vec2<f32>(BORDER + LINE);
        let medallion = abs(q.x) / field_size.x + abs(q.y) / field_size.y;
        if medallion < 0.62 {
            color = RUG_CREAM;
        }
        if medallion < 0.56 {
            color = RUG_INDIGO;
        }
        if medallion < 0.32 {
            color = RUG_OCHRE;
        }
        if medallion < 0.14 {
            color = field;
        }
        if medallion > 0.75 {
            // Small diamonds scattered over the rest of the field.
            let cell = fract(q / 0.16) - 0.5;
            if abs(cell.x) + abs(cell.y) < 0.14 {
                color = RUG_CREAM;
            }
        }
    }

    // Dye lots change along the rug, and each knot differs a little.
    color *= mix(0.85, 1.08, noise(vec2<f32>(q.y * 3.0, 0.5), s + 1.0));
    color *= mix(0.88, 1.05, hash1(floor(p / KNOT), s + 2.0));

    // The sides wore through the binding down to the foundation threads.
    let side = sign(p.x);
    let threads = noise(vec2<f32>(p.y * 30.0, side), s + 11.0) * noise(vec2<f32>(p.y * 5.0, side), s + 21.0);
    let fray = (1.0 - smoothstep(0.004, 0.012, h.x - abs(p.x))) * smoothstep(0.25, 0.4, threads);
    color = mix(color, RUG_WARP, fray * 0.6);

    // Years of use dulled the dyes and yellowed the pale wool.
    let pale = smoothstep(0.3, 0.5, max(color.r, max(color.g, color.b)));
    color *= mix(vec3<f32>(1.0), vec3<f32>(0.9, 0.82, 0.68), pale * 0.6);
    color = mix(color, RUG_DIRT, 0.2 * smoothstep(0.3, 0.8, noise(p * 3.0, s + 22.0)));

    // Feet, the chair and the way out wore the pile flat and the dye pale.
    let feet = blob(p, vec2<f32>(0.0, -0.66), vec2<f32>(0.42, 0.16), s + 12.0);
    let chair = blob(p, vec2<f32>(0.02, -0.3), vec2<f32>(0.34, 0.28), s + 13.0);
    let path = (1.0 - smoothstep(0.08, 0.2, abs(p.x - 0.12 - 0.05 * sin(p.y * 4.0))))
        * smoothstep(-0.4, -0.1, p.y);
    let middle = 1.0 - smoothstep(0.0, 0.8, length(p / h));
    let traffic = max(max(feet, chair * 0.8), max(path * 0.6, middle * 0.5));
    let worn = traffic * mix(0.6, 1.0, noise(p * 9.0, s + 3.0));
    color = mix(color, color * 0.55 + RUG_CREAM * 0.22, worn * 0.8);

    // Dirt from shoes along the paths, and grime gathering towards the edges.
    let tracked = traffic * smoothstep(0.45, 0.8, noise(p * 14.0, s + 14.0));
    let rim = (1.0 - smoothstep(0.0, 0.3, edge)) * mix(0.5, 1.0, noise(p * 7.0, s + 15.0));
    color = mix(color, RUG_DIRT, clamp(tracked * 0.4 + rim * 0.5, 0.0, 1.0));

    // A coffee spill with a darker dried rim, and drops next to it.
    let spill = max(
        blob(p, vec2<f32>(0.33, -0.52), vec2<f32>(0.075, 0.06), s + 16.0),
        max(
            blob(p, vec2<f32>(0.42, -0.44), vec2<f32>(0.018, 0.016), s + 17.0),
            blob(p, vec2<f32>(0.27, -0.42), vec2<f32>(0.012, 0.012), s + 18.0),
        ),
    );
    let ring = spill * (1.0 - spill) * 4.0;
    color = mix(color, color * COFFEE, spill * 0.7);
    color = mix(color, RUG_DIRT, ring * 0.35);

    // An old grease stain, darker and a little shiny.
    let grease = blob(p, vec2<f32>(-0.34, -0.06), vec2<f32>(0.1, 0.08), s + 19.0);
    color = mix(color, color * 0.45, grease * 0.6);

    // Lint and fluff caught in the pile.
    let lint = specks(p, 0.012, 0.02, 0.0005, 0.003, s + 20.0, pixel);
    color = mix(color, RUG_LINT, lint * 0.6);

    var out: Rug;
    out.color = color;
    out.worn = clamp(worn, 0.0, 1.0);
    out.gloss = grease * 0.5;
    return out;
}

/// A soft, ragged patch around `center` with radii `r`. 1 inside, 0 outside.
fn blob(p: vec2<f32>, center: vec2<f32>, r: vec2<f32>, s: f32) -> f32 {
    let d = length((p - center) / r);
    let ragged = d + (noise((p - center) / max(r.x, r.y) * 3.0, s) - 0.5) * 0.5;
    return 1.0 - smoothstep(0.75, 1.0, ragged);
}

/// The bumps of the knots, fading out where they get smaller than a pixel.
fn rug_height(p: vec2<f32>, pixel: f32) -> f32 {
    let waves = sin(p * TAU / KNOT);
    return 0.0003 * waves.x * waves.y * visible(KNOT, pixel);
}
