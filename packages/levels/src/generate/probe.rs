use std::collections::{HashSet, VecDeque};

use ld60_devices::{AnyDevice, Device, MAX_STEPS, Readout, RunError};

use crate::{Level, TEST_SEEDS, device, initial_rom};

const MAX_KEYS: usize = 6;
/// Key presses tried before the search gives up.
const MAX_PRESSES: usize = 4096;

/// The shortest key sequence after which a mutant shows something else than
/// the working firmware.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Probe {
    pub(crate) keys: String,
}

/// Both devices after `keys`, which both show the same.
struct Node {
    working: AnyDevice,
    mutant: AnyDevice,
    keys: String,
}

impl Probe {
    /// Searches breadth first, so the result has the fewest keys.
    ///
    /// Returns `None` if no sequence of up to [`MAX_KEYS`] keys tells the two
    /// apart within the search budget.
    pub(crate) fn find(level: &Level, working: &str, mutant: &str) -> Option<Self> {
        let boot = |firmware| {
            let mut device = device(level, initial_rom(level, firmware).ok()?).ok()?;
            device.power_on(TEST_SEEDS[0]);
            Some(device)
        };
        let (mut working, mut mutant) = (boot(working)?, boot(mutant)?);
        let shown = run(&mut working).ok()?;
        let mutant_shown = run(&mut mutant);
        if differs(&shown, mutant_shown.as_ref()) {
            return Some(Self {
                keys: String::new(),
            });
        }
        let keys: Vec<char> = level.board().keys.iter().map(|&(key, _)| key).collect();
        let mut seen = HashSet::from([state(&working, &mutant)]);
        let mut queue = VecDeque::from([Node {
            working,
            mutant,
            keys: String::new(),
        }]);
        let mut presses = 0;
        while let Some(node) = queue.pop_front() {
            if node.keys.len() == MAX_KEYS {
                continue;
            }
            for &key in &keys {
                presses += 1;
                if presses > MAX_PRESSES {
                    return None;
                }
                let (mut working, mut mutant) = (node.working.clone(), node.mutant.clone());
                let Ok(shown) = press(&mut working, key) else {
                    continue;
                };
                let mutant_shown = press(&mut mutant, key);
                let keys = format!("{}{key}", node.keys);
                if differs(&shown, mutant_shown.as_ref()) {
                    return Some(Self { keys });
                }
                if seen.insert(state(&working, &mutant)) {
                    queue.push_back(Node {
                        working,
                        mutant,
                        keys,
                    });
                }
            }
        }
        None
    }
}

fn run(device: &mut AnyDevice) -> Result<Readout, RunError> {
    device.run_until_halt(MAX_STEPS).map(|_| device.readout())
}

fn press(device: &mut AnyDevice, key: char) -> Result<Readout, RunError> {
    device.press_key(key)?;
    run(device)
}

/// Compares the text on the display, like the test runner does.
fn differs(working: &Readout, mutant: Result<&Readout, &RunError>) -> bool {
    !mutant.is_ok_and(|mutant| mutant.to_string() == working.to_string())
}

/// Everything that decides what both devices do next.
fn state(working: &AnyDevice, mutant: &AnyDevice) -> Vec<u8> {
    let mut state = Vec::new();
    for device in [working, mutant] {
        let cpu = device.cpu();
        state.extend(cpu.pc().to_le_bytes());
        for register in cpu.registers() {
            state.extend(register.value.to_le_bytes());
        }
        state.extend(cpu.ram().cells());
        state.extend(device.readout().to_string().bytes());
        state.push(0xFF);
    }
    state
}
