import package::geometry::noise::{hash1, noise};

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
