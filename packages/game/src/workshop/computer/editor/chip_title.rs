use bevy::prelude::*;

use crate::workshop::job::Job;

/// The editor header text that names the chip of the job.
#[derive(Component, Debug)]
pub(super) struct ChipTitle;

impl ChipTitle {
    pub(super) fn update(job: Res<Job>, mut titles: Query<&mut Text, With<ChipTitle>>) {
        for mut title in &mut titles {
            title.0 = job.level.chip.isa().name().to_uppercase();
        }
    }
}
