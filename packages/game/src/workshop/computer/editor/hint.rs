use bevy::prelude::*;
use ld60_locale::{Localized, Message};

use super::dump::Dump;
use crate::workshop::job::Job;
use crate::workshop::levels::Levels;
use crate::workshop::run::Origin;

/// The hint of a tutorial job, under the editor.
#[derive(Component, Debug)]
pub(super) struct HintLine;

impl HintLine {
    /// Shows the hint of a tutorial job once its firmware is in the editor.
    pub(super) fn update(
        job: Option<Res<Job>>,
        dump: Res<Dump>,
        mut lines: Query<(&mut Localized, &mut Node), With<HintLine>>,
    ) {
        let hint = job
            .filter(|job| *dump == Dump::Source && matches!(job.origin, Origin::Lesson { .. }))
            .filter(|job| job.level.firmware.is_some())
            .map(|job| Levels::text(&job.level, "hint"));
        for (mut line, mut node) in &mut lines {
            let display = if hint.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
            if let Some(hint) = &hint {
                line.set_if_neq(Message::clone(hint).into());
            }
        }
    }
}
