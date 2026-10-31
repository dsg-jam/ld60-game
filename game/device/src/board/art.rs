//! Drawing the face of a board a little each frame, so that a new board
//! revision does not hold up the game. It works the same on the web, where
//! everything runs on one thread.

use std::collections::VecDeque;
use std::mem;
use std::task::Poll;

use bevy::platform::time::Instant;
use bevy::prelude::*;
use ld60_game_parts::pcb::{Developing, Pad, Pcb, PcbArt, PcbMaterial, Router, Routing};
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

use super::Net;

/// How long the drawing may take each frame, in seconds.
const BUDGET: f32 = 0.003;
/// States the router looks at between checks of the time.
const SEARCH_STEPS: usize = 1_500;
/// Rows of the image made between checks of the time.
const IMAGE_ROWS: usize = 16;

/// The face of a board while it is drawn: its traces are routed one after
/// the other, then the image is made row by row.
pub(super) struct ArtJob {
    stage: Stage,
    router: Router,
    /// The pads of each net still to join, in order, the first of each net
    /// marked.
    pads: VecDeque<(Pad, Vec2, Net, bool)>,
    /// The pads of the net being routed that it reaches so far.
    joined: Vec<Vec2>,
    /// Picks where power and ground go down into the board.
    rng: SmallRng,
}

enum Stage {
    Routing {
        art: PcbArt,
        routing: Option<(Routing, Vec2, Net)>,
    },
    Developing(Developing),
    Done,
}

impl ArtJob {
    /// Routes `nets` on `router`, drawing the traces on `art`. Each net
    /// lists its pads, which are joined in order. `seed` picks where power
    /// goes down into the board.
    pub(super) fn new(
        art: PcbArt,
        router: Router,
        nets: impl IntoIterator<Item = (Net, Vec<(Pad, Vec2)>)>,
        seed: u64,
    ) -> Self {
        let pads = nets
            .into_iter()
            .flat_map(|(net, pads)| {
                pads.into_iter()
                    .enumerate()
                    .map(move |(index, (pad, center))| (pad, center, net, index == 0))
            })
            .collect();
        Self {
            stage: Stage::Routing { art, routing: None },
            router,
            pads,
            joined: Vec::new(),
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    /// Works on the drawing until `deadline`, and a little at least.
    /// Returns the image once it is done.
    pub(super) fn work(&mut self, deadline: Instant) -> Option<Image> {
        loop {
            self.stage = match mem::replace(&mut self.stage, Stage::Done) {
                Stage::Routing {
                    mut art,
                    routing: Some((mut routing, center, net)),
                } => match self.router.search(&mut routing, SEARCH_STEPS) {
                    Poll::Pending => Stage::Routing {
                        art,
                        routing: Some((routing, center, net)),
                    },
                    Poll::Ready(path) => {
                        if let Some(path) = path {
                            self.draw(&mut art, &path, net);
                            self.joined.push(center);
                        }
                        Stage::Routing { art, routing: None }
                    }
                },
                Stage::Routing { art, routing: None } => match self.pads.pop_front() {
                    Some((pad, center, net, true)) => {
                        self.router.seed(pad, net.id());
                        self.joined = vec![center];
                        Stage::Routing { art, routing: None }
                    }
                    Some((pad, center, net, false)) => Stage::Routing {
                        art,
                        routing: self
                            .router
                            .start(pad, center, net.id(), &self.joined)
                            .map(|routing| (routing, center, net)),
                    },
                    None => Stage::Developing(art.develop()),
                },
                Stage::Developing(mut developing) => {
                    if developing.step(IMAGE_ROWS) {
                        return Some(developing.image());
                    }
                    Stage::Developing(developing)
                }
                Stage::Done => return None,
            };
            if Instant::now() >= deadline {
                return None;
            }
        }
    }

    /// Draws the trace of `net` along `path`.
    fn draw(&mut self, art: &mut PcbArt, path: &[Vec2], net: Net) {
        art.trace(path, net.width());
        // Power and ground go down to the planes inside here and there.
        if matches!(net, Net::Power | Net::Ground) && path.len() > 2 && self.rng.random_bool(0.5) {
            art.via(path[path.len() / 2], 0.0005, 0.00025);
        }
    }
}

/// The boards whose faces are still drawn, each with the material that
/// shows it once it is done.
#[derive(Default)]
pub(super) struct ArtJobs(Vec<(ArtJob, Handle<PcbMaterial>)>);

impl ArtJobs {
    /// Draws `job`, then shows it with `material`.
    pub(super) fn push(&mut self, job: ArtJob, material: Handle<PcbMaterial>) {
        self.0.push((job, material));
    }

    /// Works on the boards in turn for at most [`BUDGET`].
    pub(super) fn work(&mut self, images: &mut Assets<Image>, pcbs: &mut Assets<PcbMaterial>) {
        let deadline = Instant::now() + std::time::Duration::from_secs_f32(BUDGET);
        while let Some((job, material)) = self.0.first_mut() {
            let Some(image) = job.work(deadline) else {
                return;
            };
            if let Some(mut pcb) = pcbs.get_mut(&*material) {
                *pcb = Pcb::material(images.add(image));
            }
            self.0.remove(0);
            if Instant::now() >= deadline {
                return;
            }
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
