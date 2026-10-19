use ld60_devices::DeviceKind;
use rand::{Rng, RngExt};

/// Each recent repair of a device takes this many percent off what the next
/// one pays.
const DROP: u32 = 8;
/// The least share of the full reward a device still pays, in percent.
const FLOOR: u32 = 40;

/// How much the locals still want each device fixed.
///
/// Every repair of a device makes the next one of its kind rarer and worth
/// less. Each time the planet sets, the locals forget half of the recent
/// repairs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Demand {
    /// Recent repairs of each device, in the order of [`DeviceKind::ALL`].
    recent: [u32; DeviceKind::ALL.len()],
}

impl Demand {
    /// The share of the full reward that `device` pays now, in percent.
    pub(crate) fn percent(&self, device: DeviceKind) -> u32 {
        100_u32
            .saturating_sub(DROP * self.recent[Self::index(device)])
            .max(FLOOR)
    }

    /// What a job on `device` with a full reward of `reward` pays now. It is
    /// rounded to the nearest unit and never below 1.
    pub(crate) fn pay(&self, device: DeviceKind, reward: u32) -> u32 {
        ((reward * self.percent(device) + 50) / 100).max(1)
    }

    /// Picks one of `signs`, each as likely as its [`Demand::percent`].
    pub(crate) fn pick(&self, signs: &[DeviceKind], rng: &mut impl Rng) -> Option<DeviceKind> {
        let total: u32 = signs.iter().map(|&device| self.percent(device)).sum();
        if total == 0 {
            return None;
        }
        let mut roll = rng.random_range(0..total);
        signs.iter().copied().find(|&device| {
            let weight = self.percent(device);
            if roll < weight {
                return true;
            }
            roll -= weight;
            false
        })
    }

    pub(crate) fn fixed(&mut self, device: DeviceKind) {
        self.recent[Self::index(device)] += 1;
    }

    pub(crate) fn recover(&mut self) {
        for recent in &mut self.recent {
            *recent /= 2;
        }
    }

    fn index(device: DeviceKind) -> usize {
        DeviceKind::ALL
            .iter()
            .position(|&kind| kind == device)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    use super::*;

    #[test]
    fn repairs_lower_the_pay_down_to_the_floor() {
        let mut demand = Demand::default();
        assert_eq!(demand.pay(DeviceKind::Tally2, 10), 10);
        demand.fixed(DeviceKind::Tally2);
        assert_eq!(demand.pay(DeviceKind::Tally2, 10), 9);
        assert_eq!(demand.pay(DeviceKind::Summa6, 10), 10);
        for _ in 0..20 {
            demand.fixed(DeviceKind::Tally2);
        }
        assert_eq!(demand.percent(DeviceKind::Tally2), FLOOR);
        assert_eq!(demand.pay(DeviceKind::Tally2, 1), 1);
    }

    #[test]
    fn demand_recovers_when_the_planet_sets() {
        let mut demand = Demand::default();
        for _ in 0..4 {
            demand.fixed(DeviceKind::Tally2);
        }
        demand.recover();
        assert_eq!(demand.percent(DeviceKind::Tally2), 100 - 2 * DROP);
    }

    #[test]
    fn worn_out_devices_come_less_often() {
        let mut demand = Demand::default();
        for _ in 0..10 {
            demand.fixed(DeviceKind::Tally2);
        }
        let mut rng = SmallRng::seed_from_u64(1);
        let signs = DeviceKind::ALL;
        let tallies = (0..1000)
            .filter(|_| demand.pick(&signs, &mut rng) == Some(DeviceKind::Tally2))
            .count();
        assert!((200..400).contains(&tallies), "{tallies}");
        assert_eq!(demand.pick(&[], &mut rng), None);
    }
}
