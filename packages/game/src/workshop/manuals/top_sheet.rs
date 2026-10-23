//! The sheets on top of the stacks on either side of the rings. A page is
//! pressed flat while other pages lie on it. Its wrinkles relax once it
//! comes to the top, and flatten again as a leaf or the cover comes down on
//! it, so no sheet ever reaches through the one on top of it.

use std::f32::consts::PI;

use bevy::prelude::*;
use ld60_core::parts::paper::{Leaf, clearance, relaxing};

use super::Filed;
use super::model::Parts;
use super::pose::Pose;
use super::stack::Stack;

/// A sheet on top of a stack, showing the wrinkles of its page.
#[derive(Component, Debug)]
pub(super) struct TopSheet {
    face_up: bool,
    /// The page it shows.
    page: Option<usize>,
    /// Seconds since the page came to the top.
    pub(super) relaxed: f32,
    /// The share of its wrinkles its mesh shows.
    shown: Option<f32>,
}

impl TopSheet {
    pub(super) fn new(face_up: bool) -> Self {
        Self {
            face_up,
            page: None,
            relaxed: 0.0,
            shown: None,
        }
    }

    /// The share of its wrinkles it shows.
    pub(super) fn share(&self) -> f32 {
        self.shown.unwrap_or(0.0)
    }

    /// Shows `page`, which comes to the top pressed flat.
    pub(super) fn show(&mut self, page: usize) {
        if self.page != Some(page) {
            self.page = Some(page);
            self.relaxed = 0.0;
            self.shown = None;
        }
    }
}

impl Stack {
    /// Relaxes the wrinkles of the sheets on top of the stacks, and presses
    /// them flat under a turning leaf and the cover.
    pub(super) fn relax_top_sheets(
        time: Res<Time>,
        files: Query<(&Stack, &Parts, &Pose, &Filed)>,
        leaves: Query<&Leaf>,
        mut sheets: Query<(&mut TopSheet, &Mesh3d)>,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        for (stack, parts, pose, filed) in &files {
            let leaf = stack
                .turning
                .as_ref()
                .and_then(|turning| leaves.get(turning.leaf).ok())
                .map(Leaf::angle);
            for (sheet, right) in [(parts.sheet, true), (parts.back_sheet, false)] {
                let Ok((mut top, mesh)) = sheets.get_mut(sheet) else {
                    continue;
                };
                top.relaxed += time.delta_secs();
                let under = leaf.map_or(1.0, |angle| {
                    clearance(if right { angle } else { PI - angle })
                });
                let share = relaxing(top.relaxed).min(under).min(pose.cover());
                if top.shown == Some(share) {
                    continue;
                }
                let Some(shape) = top.page.and_then(|page| filed.shapes.get(page)) else {
                    continue;
                };
                if let Some(mut mesh) = meshes.get_mut(mesh) {
                    *mesh = shape.leaf.lying(top.face_up, share);
                    top.shown = Some(share);
                }
            }
        }
    }
}
