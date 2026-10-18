use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
};

use super::footer::{Command, Run};
use super::{PHOSPHOR, PHOSPHOR_DIM};
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::locale::{Localized, Message};
use crate::player::focus::Focus;

pub(super) struct PagePlugin;

impl Plugin for PagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActivePage>()
            .add_systems(OnEnter(Focus::Computer), ActivePage::take_focus)
            .add_systems(Update, Page::show.run_if(resource_changed::<ActivePage>))
            .add_observer(Page::switch_on_run);
        #[cfg(feature = "debug_port")]
        app.add_debug_method("ld60.page", Page::debug_state);
    }
}

/// A page of the computer screen. Only the active one shows.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Page {
    #[default]
    Source,
    Notes,
    Versions,
}

/// The page on the screen.
#[derive(Resource, Debug, Default)]
pub(super) struct ActivePage(pub(super) Page);

/// The title bar text. It names the active page.
#[derive(Component, Debug)]
pub(super) struct PageTitle;

impl Page {
    pub(super) fn title(self) -> Message {
        Message::new(match self {
            Self::Source => "editor-title-source",
            Self::Notes => "editor-title-notes",
            Self::Versions => "editor-title-versions",
        })
    }

    /// The footer command that opens the page.
    fn command(self) -> Option<Command> {
        match self {
            Self::Source => None,
            Self::Notes => Some(Command::Notes),
            Self::Versions => Some(Command::Versions),
        }
    }

    /// The command of the active page goes back to the source.
    fn switch_on_run(run: On<Run>, mut active: ResMut<ActivePage>) {
        let Some(page) = [Self::Notes, Self::Versions]
            .into_iter()
            .find(|page| page.command() == Some(run.0))
        else {
            return;
        };
        active.0 = if active.0 == page { Self::Source } else { page };
    }

    fn show(
        active: Res<ActivePage>,
        mut pages: Query<(Entity, &Page, &mut Node)>,
        mut title: Single<&mut Localized, With<PageTitle>>,
        mut labels: Query<(&Command, &mut TextColor)>,
        state: Res<State<Focus>>,
        mut focus: ResMut<InputFocus>,
    ) {
        **title = active.0.title().into();
        for (command, mut color) in &mut labels {
            color.0 = if Some(*command) == active.0.command() {
                PHOSPHOR
            } else {
                PHOSPHOR_DIM
            };
        }
        let at_computer = *state.get() == Focus::Computer;
        for (entity, &page, mut node) in &mut pages {
            if page == active.0 {
                node.display = Display::Flex;
                if at_computer {
                    focus.set(entity, FocusCause::Navigated);
                }
            } else {
                node.display = Display::None;
            }
        }
    }

    /// `ld60.page` returns the name of the active page.
    #[cfg(feature = "debug_port")]
    fn debug_state(
        In(_): In<Option<serde_json::Value>>,
        active: Res<ActivePage>,
    ) -> bevy::remote::BrpResult {
        Ok(serde_json::json!({ "page": format!("{:?}", active.0) }))
    }
}

impl ActivePage {
    fn take_focus(
        active: Res<ActivePage>,
        pages: Query<(Entity, &Page)>,
        mut focus: ResMut<InputFocus>,
    ) {
        if let Some((entity, _)) = pages.iter().find(|(_, page)| **page == active.0) {
            focus.set(entity, FocusCause::Navigated);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::Locale;

    #[test]
    fn every_title_formats() {
        let titles = [Page::Source, Page::Notes, Page::Versions].map(Page::title);
        Locale::on_disk().assert_formats(titles);
    }
}
