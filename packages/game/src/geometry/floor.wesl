import package::geometry::noise::{TAU, hash1, noise, visible};

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

/// A knotted rug with a border and a medallion, lying flat with its length
/// along y and `field` as the main color. `h` is its half size.
fn rug_color(p: vec2<f32>, h: vec2<f32>, field: vec3<f32>, s: f32) -> vec3<f32> {
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
    // Feet wore down the middle.
    let worn = (1.0 - smoothstep(0.0, 0.8, length(p / h))) * smoothstep(0.3, 0.7, noise(p * 6.0, s + 3.0));
    return mix(color, color * 0.6 + RUG_CREAM * 0.15, worn * 0.6);
}

/// The bumps of the knots, fading out where they get smaller than a pixel.
fn rug_height(p: vec2<f32>, pixel: f32) -> f32 {
    let waves = sin(p * TAU / KNOT);
    return 0.0003 * waves.x * waves.y * visible(KNOT, pixel);
}
