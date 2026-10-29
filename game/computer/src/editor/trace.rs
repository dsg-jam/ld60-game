use bevy::prelude::*;

use super::footer::{Command, Run};

/// Whether the editor shows the last lines the chip ran.
#[derive(Resource, Debug, PartialEq, Eq)]
pub(super) struct TraceView(pub(super) bool);

impl Default for TraceView {
    fn default() -> Self {
        Self(true)
    }
}

impl TraceView {
    pub(super) fn toggle_on_run(run: On<Run>, mut view: ResMut<TraceView>) {
        if run.0 == Command::Trace {
            view.0 = !view.0;
        }
    }
}
