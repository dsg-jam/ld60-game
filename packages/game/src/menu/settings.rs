use bevy::prelude::*;
use ld60_core::settings::{Setting, Settings};
use ld60_locale::{Localized, Message};

use super::list::{Action, Entries, Entry};

/// Room for the name of a setting, so the values line up.
const LABEL_WIDTH: f32 = 340.0;

pub(super) struct SettingsPagePlugin;

impl Plugin for SettingsPagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, SettingValue::show);
    }
}

/// The settings page.
pub(super) struct SettingsPage;

/// The text that shows the value of a [`Setting`].
#[derive(Component, Debug)]
struct SettingValue(Setting);

impl SettingsPage {
    pub(super) fn bundle() -> impl Bundle {
        let rows: Vec<_> = Setting::ALL
            .into_iter()
            .map(|setting| {
                Entry::bundle(
                    Action::Setting(setting),
                    (
                        Node::default(),
                        Pickable::IGNORE,
                        children![
                            (
                                Node {
                                    min_width: px(LABEL_WIDTH),
                                    ..default()
                                },
                                Entries::label(Message::new(setting.label())),
                            ),
                            (
                                SettingValue(setting),
                                Entries::label(Localized::verbatim("")),
                            ),
                        ],
                    ),
                )
            })
            .collect();
        (
            Entries,
            Children::spawn((
                SpawnIter(rows.into_iter()),
                Spawn(Entry::bundle(
                    Action::Back,
                    Entries::label(Message::new("menu-back")),
                )),
            )),
        )
    }
}

impl SettingValue {
    fn show(settings: Res<Settings>, mut values: Query<(&SettingValue, &mut Localized)>) {
        for (value, mut text) in &mut values {
            text.set_if_neq(settings.value(value.0).into());
        }
    }
}
