use std::ops::RangeInclusive;

use ld60_calco::RAM_LEN;
use ld60_devices::Fault;
use ld60_machine::StuckBit;

use super::probe::Probe;
use super::{Bug, Generated};
use crate::variant::rng::Rng;
use crate::{Difficulty, JobText, Operator, Symptom, TestCase, Variant, initial_rom};

/// Cells, bits and free cells tried before the job gets ordinary bugs.
const TRIES: usize = 24;
/// The job pays like one with this many bugs.
const PAYS_AS_BUGS: usize = 2;

impl Generated {
    /// A job whose only bug is a stuck RAM bit in a cell that `working`
    /// uses. The fix moves the cell to one the firmware leaves free.
    ///
    /// The test that shows the fault has a length within `keys`. The job
    /// `seed` picks its passing tests.
    pub(super) fn stuck(
        working: &Variant,
        seed: u64,
        rng: &mut Rng,
        difficulty: &Difficulty,
        keys: &RangeInclusive<usize>,
    ) -> Option<Self> {
        let level = &working.level;
        let (direct, indexed) = working.used_cells();
        let display = level.board().display_cells();
        let faulty_cells: Vec<usize> = level
            .faults
            .iter()
            .filter_map(|fault| match fault {
                Fault::StuckRamBit(bit) => Some(usize::from(bit.cell)),
                Fault::FadedRomBits { .. } => None,
            })
            .collect();
        let pinned = |cell: &usize| {
            display.contains(cell) || indexed.contains(cell) || faulty_cells.contains(cell)
        };
        let movable: Vec<usize> = direct.iter().copied().filter(|c| !pinned(c)).collect();
        let free: Vec<usize> = (0..RAM_LEN)
            .filter(|c| !pinned(c) && !direct.contains(c))
            .collect();
        if movable.is_empty() || free.is_empty() {
            return None;
        }
        (0..TRIES).find_map(|_| {
            let from = movable[rng.below(movable.len())] as u8;
            let to = free[rng.below(free.len())] as u8;
            let bit = StuckBit {
                cell: from,
                bit: rng.below(4) as u8,
                high: rng.below(2) == 1,
            };
            Self::moved(working, seed, difficulty, keys, bit, to)
        })
    }

    /// The job for `bit` stuck in a cell of `working`, fixed by moving that
    /// cell to `to`.
    fn moved(
        working: &Variant,
        seed: u64,
        difficulty: &Difficulty,
        keys: &RangeInclusive<usize>,
        bit: StuckBit,
        to: u8,
    ) -> Option<Self> {
        let fixed = working.swap_cells(bit.cell, to);
        let moved = Variant {
            level: working.level.clone(),
            buggy: fixed.clone(),
            fixed: fixed.clone(),
        };
        if !moved.behaves_like(working) {
            return None;
        }
        let mut level = working.level.clone();
        level.faults.push(Fault::StuckRamBit(bit));
        level.remarks.push(JobText::SPILL_REMARK.into());
        let faulty = Variant {
            level: level.clone(),
            ..moved.clone()
        };
        if !faulty.behaves_like(&moved) || !faulty.passes_line_tests() {
            return None;
        }
        let probe = Probe::find(&level, &fixed, &working.fixed, *keys.end())?;
        if !keys.contains(&probe.keys.len()) {
            return None;
        }
        let fixed_rom = initial_rom(&level, &fixed).ok()?;
        let buggy_rom = initial_rom(&level, &working.fixed).ok()?;
        let case = TestCase::shown(&level, &fixed_rom, &probe.keys)?;
        if !case.fails_everywhere(&level, &buggy_rom) {
            return None;
        }
        let symptom = Symptom::shown(&level, &fixed_rom, &buggy_rom, &case.keys)?;
        let mut tests = vec![case];
        let mut rng = Rng::new(seed, &format!("stuck tests {}", bit.cell));
        if !TestCase::add_passing(&level, &fixed_rom, &buggy_rom, &mut tests, &mut rng) {
            return None;
        }
        level.tests = tests;
        level.reward = difficulty.reward(PAYS_AS_BUGS);
        let line = working
            .fixed
            .lines()
            .zip(fixed.lines())
            .position(|(a, b)| a != b)
            .unwrap_or_default();
        let variant = Variant {
            level,
            buggy: working.fixed.clone(),
            fixed,
        };
        variant.verify().ok()?;
        Some(Self {
            variant,
            bugs: vec![Bug {
                operator: Operator::MovedCell,
                line,
                symptom,
                test: 0,
            }],
        })
    }
}
