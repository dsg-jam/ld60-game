use bevy::prelude::*;
use ld60_game_run::job::Job;
use ld60_locale::Localized;

/// The editor header text that names the chip of the job.
#[derive(Component, Debug)]
pub(super) struct ChipTitle;

impl ChipTitle {
    /// Shown without a job.
    pub(super) const NONE: &str = "CALCO";

    pub(super) fn update(
        job: Option<Res<Job>>,
        mut titles: Query<&mut Localized, With<ChipTitle>>,
    ) {
        let name = job.map_or_else(
            || Self::NONE.into(),
            |job| job.level.chip.isa().name().to_uppercase(),
        );
        for mut title in &mut titles {
            title.set_if_neq(Localized::verbatim(name.clone()));
        }
    }
}
