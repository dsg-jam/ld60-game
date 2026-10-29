use ld60_devices::{AnyDevice, Device, Readout, RunError};

use super::Variant;
use super::rng::Rng;
use crate::{Level, device, initial_rom};

const SEQUENCES: u64 = 16;
const KEYS_PER_SEQUENCE: usize = 8;
/// Far more than any key needs. Firmware that hangs ends the sequence.
const STEPS: u32 = 4096;

impl Variant {
    /// Checks that both firmware files show the same as in `original` for a
    /// fixed set of random key sequences.
    ///
    /// RAM starts zeroed, so renamed cells hold the same values.
    pub(crate) fn behaves_like(&self, original: &Self) -> bool {
        let keys: Vec<char> = original
            .level
            .board()
            .keys
            .iter()
            .map(|&(key, _)| key)
            .collect();
        let sequences: Vec<Vec<char>> = (0..SEQUENCES)
            .map(|n| {
                let mut rng = Rng::new(n, "keys");
                (0..KEYS_PER_SEQUENCE)
                    .map(|_| keys[(rng.next_u64() % keys.len() as u64) as usize])
                    .collect()
            })
            .collect();
        [
            (&self.buggy, &original.buggy),
            (&self.fixed, &original.fixed),
        ]
        .into_iter()
        .all(|(mine, theirs)| {
            let (Some(mut mine), Some(mut theirs)) =
                (boot(&self.level, mine), boot(&original.level, theirs))
            else {
                return false;
            };
            sequences
                .iter()
                .all(|keys| trace(&mut mine, keys) == trace(&mut theirs, keys))
        })
    }
}

fn boot(level: &Level, firmware: &str) -> Option<AnyDevice> {
    device(level, initial_rom(level, firmware).ok()?).ok()
}

/// The readout after power on and after each key.
fn trace(device: &mut AnyDevice, keys: &[char]) -> Vec<Result<Readout, RunError>> {
    device.power_on(0);
    device.cpu_mut().ram_mut().fill(|| 0);
    let run = |device: &mut AnyDevice| device.run_until_halt(STEPS).map(|_| device.readout());
    let mut shown = vec![run(device)];
    for &key in keys {
        if shown.last().is_some_and(Result::is_err) {
            break;
        }
        shown.push(device.press_key(key).and_then(|()| run(device)));
    }
    shown
}

#[cfg(test)]
mod tests {
    use ld60_devices::{BoardRev, DeviceKind};

    use super::*;
    use crate::Catalog;
    use crate::variant::firmware::Firmware;

    fn moved_to_b(firmware: &str) -> (Variant, Variant) {
        let catalog = Catalog::in_repository();
        let source = catalog
            .levels()
            .iter()
            .find(|s| s.path.starts_with("tally2"))
            .unwrap();
        let original = Variant {
            level: source.load().unwrap(),
            buggy: firmware.into(),
            fixed: firmware.into(),
        };
        let mut rewired = Firmware::new(firmware);
        let boards = DeviceKind::Tally2.boards();
        rewired.rewire(&boards[0], &boards[1]).unwrap();
        let mut level = original.level.clone();
        level.board = BoardRev::B;
        let moved = Variant {
            level,
            buggy: rewired.to_string(),
            fixed: rewired.to_string(),
        };
        (original, moved)
    }

    #[test]
    fn accepts_a_faithful_rewrite() {
        let (original, moved) = moved_to_b(
            "main: WAIT\nIN KEY\nCPI 0x0A\nJZ plus\nJMP main\nplus: LD M2\nINC\nST M2\nJMP main\n",
        );
        assert!(moved.behaves_like(&original));
    }

    #[test]
    fn rejects_a_key_code_that_passes_through_ram() {
        let (original, moved) = moved_to_b(
            "main: WAIT\nIN KEY\nST M7\nLD M7\nCPI 0x0A\nJZ plus\nJMP main\nplus: LD M2\nINC\nST M2\nJMP main\n",
        );
        assert!(!moved.behaves_like(&original));
    }
}
