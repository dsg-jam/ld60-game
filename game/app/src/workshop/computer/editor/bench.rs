use bevy::prelude::*;
use ld60_devices::Rom;
use ld60_levels::Level;
use ld60_locale::{Localized, Message};

use super::footer::{Command, Run, Status};
use crate::device::{Connected, Emulator};
use crate::workshop::job::Job;
use crate::workshop::pickup::Complaint;

/// What the test bench found when it tried the device like the customer
/// will.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Verdict(Option<Complaint>);

impl Verdict {
    /// Runs the customer's tests of `level` against `rom`.
    pub(super) fn new(level: &Level, rom: &Rom) -> Self {
        Self(Complaint::find(level, rom))
    }

    /// Tests the firmware in the device on the bench.
    pub(super) fn run_on_command(
        run: On<Run>,
        job: Option<Res<Job>>,
        emulators: Query<&Emulator, With<Connected>>,
        mut status: Single<&mut Localized, With<Status>>,
    ) {
        if run.0 != Command::Test {
            return;
        }
        let message = match (job, emulators.single()) {
            (None, _) => Message::new("editor-no-job"),
            (_, Err(_)) => Message::new("editor-no-device"),
            (Some(job), Ok(emulator)) => Self::new(&job.level, emulator.rom()).into(),
        };
        **status = message.into();
    }
}

impl From<Verdict> for Message {
    fn from(verdict: Verdict) -> Self {
        let Some(complaint) = verdict.0 else {
            return Self::new("editor-bench-pass");
        };
        match complaint {
            Complaint::Dead => Self::new("editor-bench-dead"),
            Complaint::Wrong {
                keys,
                expected,
                shown,
                ..
            } => {
                let id = if keys.is_empty() {
                    "editor-bench-wrong-start"
                } else {
                    "editor-bench-wrong"
                };
                Self::new(id)
                    .with("keys", keys)
                    .with("shown", shown)
                    .with("expected", expected)
            }
            Complaint::Froze { keys } if keys.is_empty() => Self::new("editor-bench-froze-start"),
            Complaint::Froze { keys } => Self::new("editor-bench-froze").with("keys", keys),
        }
    }
}

#[cfg(test)]
impl Verdict {
    fn passed(&self) -> bool {
        self.0.is_none()
    }
}

#[cfg(test)]
mod tests {
    use ld60_levels::{assemble, burn, initial_rom};
    use ld60_locale::Locale;

    use super::*;
    use crate::workshop::levels::Levels;

    #[test]
    fn passes_only_the_fixed_firmware() {
        let levels = Levels::on_disk();
        for source in levels.levels() {
            let job = ld60_levels::Variant::new(source, ld60_levels::GAME_SEED).unwrap();
            let buggy = initial_rom(&job.level, &job.buggy).unwrap();
            let image = assemble(&job.level, &job.fixed).unwrap().image;
            let fixed = burn(&job.level, &buggy, image).unwrap();
            assert!(
                !Verdict::new(&job.level, &buggy).passed(),
                "{}",
                job.level.id
            );
            assert!(
                Verdict::new(&job.level, &fixed).passed(),
                "{}",
                job.level.id
            );
        }
    }

    #[test]
    fn every_verdict_formats() {
        let verdicts = Complaint::examples()
            .into_iter()
            .map(Some)
            .chain([None])
            .map(|complaint| Message::from(Verdict(complaint)));
        Locale::on_disk().assert_formats(verdicts);
    }
}
