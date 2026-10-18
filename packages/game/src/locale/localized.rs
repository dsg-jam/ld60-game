use bevy::prelude::*;

use super::Locale;

/// Fills the [`Text`] of its entity with a message in the player's
/// language.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(Text)]
pub(crate) struct Localized(pub(crate) &'static str);

impl Localized {
    pub(super) fn show(locale: Res<Locale>, mut texts: Query<(Ref<Localized>, &mut Text)>) {
        for (localized, mut text) in &mut texts {
            if locale.is_changed() || localized.is_changed() {
                text.0 = locale.text(localized.0);
            }
        }
    }
}
