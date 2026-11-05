use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::Walker;
use super::outing::Activity;

/// People who meet on the sidewalk this close may stop to chat, with
/// this chance each second they are that close, once each has walked
/// this far since the last time.
const MEET: Vec2 = Vec2::new(1.3, 1.0);
const CHAT_CHANCE: f32 = 0.5;
pub(super) const CHAT_EVERY: Range<f32> = 30.0..120.0;
/// Seconds they chat for.
pub(super) const CHAT_FOR: Range<f32> = 6.0..20.0;
/// Seconds between phrases while chatting.
pub(super) const CHAT_TALK: Range<f32> = 1.5..5.0;
/// Of those who chatted, this share then sit down side by side on two
/// free stools this close together.
const SIT_TOGETHER: f64 = 0.35;
/// Seated people chat with neighbours this close, turning to them now
/// and then, every this many seconds, for this many.
pub(super) const NEIGHBOUR: f32 = 1.7;
pub(super) const TURN_EVERY: Range<f32> = 3.0..9.0;
pub(super) const TURN_FOR: Range<f32> = 2.0..5.0;

/// Someone strolling, as seen when picking who stops to chat.
#[derive(Debug, Clone, Copy)]
pub(super) struct Passer {
    pub(super) at: Vec2,
    /// Whether it is strolling and may stop to chat.
    pub(super) free: bool,
}

/// Two people stopping to chat, as seen by one of them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Meeting {
    /// The index of the other.
    pub(super) with: usize,
    /// Seconds they chat for.
    pub(super) left: f32,
    /// Whether they go and sit down together afterwards.
    pub(super) together: bool,
}

impl Passer {
    /// Who stops to chat with whom in the next `dt` seconds, for each of
    /// `passers`. Only pairs that both are free come about.
    pub(super) fn pairs(passers: &[Passer], dt: f32, rng: &mut impl Rng) -> Vec<Option<Meeting>> {
        let chance = f64::from((CHAT_CHANCE * dt).clamp(0.0, 1.0));
        let mut partners = vec![None; passers.len()];
        for (a, first) in passers.iter().enumerate() {
            if !first.free || partners[a].is_some() {
                continue;
            }
            let partner = passers.iter().enumerate().skip(a + 1).find(|(b, second)| {
                let apart = (second.at - first.at).abs();
                second.free && partners[*b].is_none() && apart.x < MEET.x && apart.y < MEET.y
            });
            if let Some((b, _)) = partner
                && rng.random_bool(chance)
            {
                let left = rng.random_range(CHAT_FOR);
                let together = rng.random_bool(SIT_TOGETHER);
                let meeting = |with| Meeting {
                    with,
                    left,
                    together,
                };
                partners[a] = Some(meeting(b));
                partners[b] = Some(meeting(a));
            }
        }
        partners
    }
}

impl Walker {
    /// Whether it strolls and has walked far enough since it last
    /// chatted to stop for a chat.
    pub(super) fn passer(&self) -> Passer {
        Passer {
            at: Vec2::new(self.x, self.z),
            free: self.activity.is_stroll() && self.until_chat <= 0.0 && self.stopped <= 0.0,
        }
    }

    /// Stops to chat with someone at `with` for `left` seconds, and then
    /// goes to sit on a stool from a side, if it has one.
    pub(super) fn chat_with(
        &mut self,
        with: Vec2,
        left: f32,
        then: Option<(usize, f32)>,
        rng: &mut impl Rng,
    ) {
        self.until_chat = rng.random_range(CHAT_EVERY);
        self.activity = Activity::Chat {
            with,
            left,
            heading: self.heading,
            then,
        };
    }

    /// The turn of the head towards `at`, from where it stands.
    pub(super) fn turn_to(&self, at: Vec2) -> Vec2 {
        let way = at - Vec2::new(self.x, self.z);
        let local = Quat::from_rotation_y(-self.yaw) * Vec3::new(way.x, 0.0, way.y);
        Vec2::new(local.x.atan2(local.z), 0.05)
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    use super::*;

    fn passer(x: f32, free: bool) -> Passer {
        Passer {
            at: Vec2::new(x, -2.0),
            free,
        }
    }

    #[test]
    fn pairs_go_both_ways() {
        let passers = [passer(0.0, true), passer(0.8, true), passer(5.0, true)];
        let partners = Passer::pairs(&passers, 10.0, &mut SmallRng::seed_from_u64(1));
        let with: Vec<_> = partners
            .iter()
            .map(|meeting| meeting.map(|m| m.with))
            .collect();
        assert_eq!(with, [Some(1), Some(0), None]);
        let [Some(a), Some(b), _] = partners[..] else {
            panic!("no pair");
        };
        assert_eq!((a.left, a.together), (b.left, b.together));
    }

    #[test]
    fn busy_people_do_not_stop() {
        let passers = [passer(0.0, true), passer(0.8, false)];
        let partners = Passer::pairs(&passers, 10.0, &mut SmallRng::seed_from_u64(1));
        assert!(partners.iter().all(Option::is_none));
    }
}
