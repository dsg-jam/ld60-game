use std::hash::Hash;

use bevy::{platform::hash::fixed_hash_one, prelude::*};
use serde::Deserialize;

use super::stock::Stock;

/// The shader gets one of this many print runs.
const RUNS: u64 = 1000;

/// A way of printing a page, which leaves its own marks in the ink.
///
/// Matches the constants in `press.wesl`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Press {
    /// The shop's own impact printer, for everything the bench prints.
    #[default]
    DotMatrix = 0,
    /// Printed in a print shop, with crisp, dense ink.
    Offset = 1,
    Photocopy = 2,
    /// A spirit duplicator, which copies in purple dye.
    Spirit = 3,
    /// Inked type pressed into the paper.
    Letterpress = 4,
    Typewriter = 5,
}

/// How a page was printed. On a sheet, how the page it shows was printed.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct Print {
    pub(super) press: Press,
    /// Tells print runs on the same kind of press apart.
    pub(super) run: f32,
    pub(super) stock: Stock,
}

impl Print {
    /// The same print run on `stock`.
    pub fn on(self, stock: Stock) -> Self {
        Self { stock, ..self }
    }
}

impl Press {
    /// A print run on this press, on the shop's paper until
    /// [`Print::on`] says otherwise. The same `run` always looks the same.
    pub fn run(self, run: impl Hash) -> Print {
        Print {
            press: self,
            run: (fixed_hash_one(run) % RUNS) as f32,
            stock: Stock::default(),
        }
    }
}
