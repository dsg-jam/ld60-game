use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::mem;

use bevy::prelude::*;

const FREE: u32 = 0;
const BLOCKED: u32 = u32::MAX;
/// Pads not yet joined to their net are marked from here on.
const PAD_MARKS: u32 = 1 << 30;
const STRAIGHT: u32 = 10;
const DIAGONAL: u32 = 14;
/// What a bend costs, so traces run long and straight.
const BEND: u32 = 12;
/// The eight directions, then none for the start.
const STEPS: [IVec2; 8] = [
    IVec2::new(1, 0),
    IVec2::new(1, 1),
    IVec2::new(0, 1),
    IVec2::new(-1, 1),
    IVec2::new(-1, 0),
    IVec2::new(-1, -1),
    IVec2::new(0, -1),
    IVec2::new(1, -1),
];
const START: usize = 8;

/// Routes the traces of a board on a grid, around what is in the way, in
/// straight runs and 45 degree bends.
///
/// Positions are in meters from the middle of the board, with +Y up, as in
/// [`PcbArt`](super::PcbArt).
#[derive(Debug)]
pub struct Router {
    size: Vec2,
    pitch: f32,
    cells: IVec2,
    owners: Vec<u32>,
    search: Search,
}

/// What a search found so far, per cell and heading. Kept from route to
/// route, so the board's worth of states is not filled anew each time.
#[derive(Debug, Default)]
struct Search {
    cost: Vec<u32>,
    came: Vec<u32>,
    /// The round that set each state. Older ones count as unvisited.
    seen: Vec<u32>,
    /// The round that reached each cell in [`Router::reaches`].
    flooded: Vec<u32>,
    round: u32,
}

/// A pad that a trace can start from, see [`Router::pad`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pad(u32);

impl Router {
    /// An empty board of `size`, with cells `pitch` apart. Nothing runs
    /// within `margin` of the edge.
    pub fn new(size: Vec2, pitch: f32, margin: f32) -> Self {
        let cells = (size / pitch).floor().as_ivec2().max(IVec2::ONE);
        let mut router = Self {
            size,
            pitch,
            cells,
            owners: vec![FREE; (cells.x * cells.y) as usize],
            search: Search::default(),
        };
        let inner = size - 2.0 * margin;
        for y in 0..cells.y {
            for x in 0..cells.x {
                let off = router.point(IVec2::new(x, y)).abs() - inner / 2.0;
                if off.max_element() > 0.0 {
                    router.owners[(y * cells.x + x) as usize] = BLOCKED;
                }
            }
        }
        router
    }

    /// Keeps traces out of the rectangle at `center` of `size`.
    pub fn block(&mut self, center: Vec2, size: Vec2) {
        self.mark(center, size, |_| BLOCKED);
    }

    /// The pad numbered `index` in the rectangle at `center` of `size`. Its
    /// net is set when it is seeded or routed, and other nets keep out.
    pub fn pad(&mut self, center: Vec2, size: Vec2, index: u32) -> Pad {
        let pad = Pad(PAD_MARKS + index);
        self.mark(
            center,
            size,
            |owner| {
                if owner == BLOCKED { BLOCKED } else { pad.0 }
            },
        );
        pad
    }

    /// Makes `pad` the start of `net`, which later pads join.
    pub fn seed(&mut self, pad: Pad, net: u32) {
        self.join(pad, net);
    }

    /// The trace from `from`, a pad at `start`, to the nearest part of
    /// `net`, through free cells. Returns its corners, or nothing if the
    /// way is blocked. A routed pad becomes part of `net`.
    pub fn route(
        &mut self,
        from: Pad,
        start: Vec2,
        net: u32,
        targets: &[Vec2],
    ) -> Option<Vec<Vec2>> {
        let start_cell = self.cell(start)?;
        let goals: Vec<IVec2> = targets.iter().filter_map(|&t| self.cell(t)).collect();
        let mut search = mem::take(&mut self.search);
        search.begin(self.owners.len());
        // A way that is blocked costs the search the whole board, so a
        // cheaper flood over the cells looks first.
        if !self.reaches(&mut search, start_cell, from, net) {
            self.search = search;
            return None;
        }
        let mut open = BinaryHeap::new();
        let first = self.index(start_cell) * (START + 1) + START;
        search.reach(first, 0, u32::MAX);
        open.push(Reverse((Self::guess(start_cell, &goals), first)));
        let mut goal = None;
        while let Some(Reverse((_, state))) = open.pop() {
            let (cell_index, heading) = (state / (START + 1), state % (START + 1));
            let cell = IVec2::new(
                cell_index as i32 % self.cells.x,
                cell_index as i32 / self.cells.x,
            );
            if self.owners[cell_index] == net {
                goal = Some(state);
                break;
            }
            let here = search.cost(state);
            for (direction, step) in STEPS.iter().enumerate() {
                let next = cell + *step;
                if !self.passable(next, from, net) {
                    continue;
                }
                // No squeezing between two corners.
                if step.x != 0
                    && step.y != 0
                    && !(self.passable(cell + IVec2::new(step.x, 0), from, net)
                        && self.passable(cell + IVec2::new(0, step.y), from, net))
                {
                    continue;
                }
                let mut price = if step.x != 0 && step.y != 0 {
                    DIAGONAL
                } else {
                    STRAIGHT
                };
                if heading != START && heading != direction {
                    price += BEND;
                }
                let next_state = self.index(next) * (START + 1) + direction;
                let total = here + price;
                if total < search.cost(next_state) {
                    search.reach(next_state, total, state as u32);
                    open.push(Reverse((total + Self::guess(next, &goals), next_state)));
                }
            }
        }
        let path = goal.map(|mut state| {
            let mut cells = Vec::new();
            loop {
                cells.push(state / (START + 1));
                match search.came[state] {
                    u32::MAX => break,
                    previous => state = previous as usize,
                }
            }
            cells
        });
        self.search = search;
        let mut cells = path?;
        cells.reverse();
        for &cell in &cells {
            self.owners[cell] = net;
        }
        self.join(from, net);
        Some(self.corners(start, &cells))
    }

    /// Whether a trace from `start` can get to `net` at all.
    fn reaches(&self, search: &mut Search, start: IVec2, from: Pad, net: u32) -> bool {
        let mut stack = vec![start];
        search.flooded[self.index(start)] = search.round;
        while let Some(cell) = stack.pop() {
            if self.owners[self.index(cell)] == net {
                return true;
            }
            for step in STEPS {
                let next = cell + step;
                if !self.passable(next, from, net)
                    || search.flooded[self.index(next)] == search.round
                {
                    continue;
                }
                if step.x != 0
                    && step.y != 0
                    && !(self.passable(cell + IVec2::new(step.x, 0), from, net)
                        && self.passable(cell + IVec2::new(0, step.y), from, net))
                {
                    continue;
                }
                search.flooded[self.index(next)] = search.round;
                stack.push(next);
            }
        }
        false
    }

    fn join(&mut self, pad: Pad, net: u32) {
        for owner in &mut self.owners {
            if *owner == pad.0 {
                *owner = net;
            }
        }
    }

    /// The corners of the path through `cells`, from `start`.
    fn corners(&self, start: Vec2, cells: &[usize]) -> Vec<Vec2> {
        let points: Vec<Vec2> = cells
            .iter()
            .map(|&index| {
                self.point(IVec2::new(
                    index as i32 % self.cells.x,
                    index as i32 / self.cells.x,
                ))
            })
            .collect();
        let mut corners = vec![start];
        for window in points.windows(3) {
            let (a, b, c) = (window[0], window[1], window[2]);
            if ((b - a).normalize() - (c - b).normalize()).length() > 1e-3 {
                corners.push(b);
            }
        }
        if let Some(&last) = points.last() {
            corners.push(last);
        }
        corners
    }

    /// The octile distance to the nearest of `goals`, in steps.
    fn guess(cell: IVec2, goals: &[IVec2]) -> u32 {
        goals
            .iter()
            .map(|&target| {
                let d = (target - cell).abs();
                let (long, short) = (d.max_element() as u32, d.min_element() as u32);
                STRAIGHT * (long - short) + DIAGONAL * short
            })
            .min()
            .unwrap_or_default()
    }

    fn passable(&self, cell: IVec2, from: Pad, net: u32) -> bool {
        if cell.cmplt(IVec2::ZERO).any() || cell.cmpge(self.cells).any() {
            return false;
        }
        let owner = self.owners[self.index(cell)];
        owner == FREE || owner == net || owner == from.0
    }

    fn mark(&mut self, center: Vec2, size: Vec2, owner: impl Fn(u32) -> u32) {
        for y in 0..self.cells.y {
            for x in 0..self.cells.x {
                let cell = IVec2::new(x, y);
                let off = (self.point(cell) - center).abs() - size / 2.0;
                if off.max_element() <= self.pitch / 2.0 {
                    let index = self.index(cell);
                    self.owners[index] = owner(self.owners[index]);
                }
            }
        }
    }

    fn cell(&self, point: Vec2) -> Option<IVec2> {
        let cell = ((point + self.size / 2.0) / self.pitch).floor().as_ivec2();
        (cell.cmpge(IVec2::ZERO).all() && cell.cmplt(self.cells).all()).then_some(cell)
    }

    fn point(&self, cell: IVec2) -> Vec2 {
        (cell.as_vec2() + 0.5) * self.pitch - self.size / 2.0
    }

    fn index(&self, cell: IVec2) -> usize {
        (cell.y * self.cells.x + cell.x) as usize
    }
}

impl Search {
    /// Starts a search over `cells`, all unvisited.
    fn begin(&mut self, cells: usize) {
        let states = cells * (START + 1);
        if self.seen.len() != states || self.round == u32::MAX {
            *self = Self {
                cost: vec![0; states],
                came: vec![0; states],
                seen: vec![0; states],
                flooded: vec![0; cells],
                round: 0,
            };
        }
        self.round += 1;
    }

    fn cost(&self, state: usize) -> u32 {
        if self.seen[state] == self.round {
            self.cost[state]
        } else {
            u32::MAX
        }
    }

    fn reach(&mut self, state: usize, cost: u32, from: u32) {
        self.cost[state] = cost;
        self.came[state] = from;
        self.seen[state] = self.round;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trace_goes_around_what_is_in_the_way() {
        let mut router = Router::new(Vec2::splat(0.02), 0.0005, 0.001);
        let (a, b) = (Vec2::new(-0.006, 0.0), Vec2::new(0.006, 0.0));
        let start = router.pad(a, Vec2::splat(0.001), 0);
        let end = router.pad(b, Vec2::splat(0.001), 1);
        router.seed(end, 1);
        router.block(Vec2::ZERO, Vec2::new(0.002, 0.008));
        let path = router.route(start, a, 1, &[b]).unwrap();
        assert_eq!(path[0], a);
        // It ends on the pad, which reaches to the next cell around it.
        assert!(path.last().unwrap().distance(b) < 0.0015, "{path:?}");
        assert!(path.iter().any(|point| point.y.abs() > 0.004));
    }

    #[test]
    fn a_blocked_trace_is_not_routed() {
        let mut router = Router::new(Vec2::splat(0.02), 0.0005, 0.001);
        let (a, b) = (Vec2::new(-0.006, 0.0), Vec2::new(0.006, 0.0));
        let start = router.pad(a, Vec2::splat(0.001), 0);
        let end = router.pad(b, Vec2::splat(0.001), 1);
        router.seed(end, 1);
        router.block(Vec2::ZERO, Vec2::new(0.002, 0.02));
        assert!(router.route(start, a, 1, &[b]).is_none());
    }
}
