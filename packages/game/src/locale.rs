mod localized;
mod messages;

use std::sync::Arc;

use bevy::prelude::*;
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use unic_langid::{LanguageIdentifier, langid};

pub(crate) use self::localized::Localized;
use self::messages::{Messages, MessagesLoader};

const LANGUAGE: LanguageIdentifier = langid!("en");
/// The Fluent files of each language, in `locales/<language>/`.
const FILES: &[&str] = &["ui.ftl"];

pub(crate) struct LocalePlugin;

impl Plugin for LocalePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Messages>()
            .init_asset_loader::<MessagesLoader>()
            .init_resource::<Locale>()
            .add_systems(Update, (Locale::assemble, Localized::show).chain());
    }
}

/// The messages of the player's language.
#[derive(Resource)]
pub(crate) struct Locale {
    language: LanguageIdentifier,
    files: Vec<Handle<Messages>>,
    /// Empty until all files are loaded.
    bundle: Option<FluentBundle<Arc<FluentResource>>>,
}

impl FromWorld for Locale {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        let files = FILES
            .iter()
            .map(|file| assets.load(format!("locales/{LANGUAGE}/{file}")))
            .collect();
        Self {
            language: LANGUAGE,
            files,
            bundle: None,
        }
    }
}

impl Locale {
    /// The message `id`. It's empty until the messages are loaded, and shows
    /// the ID if there is no such message.
    pub(crate) fn text(&self, id: &str) -> String {
        let Some(bundle) = &self.bundle else {
            return String::new();
        };
        let Some(pattern) = bundle.get_message(id).and_then(|message| message.value()) else {
            warn!("no message `{id}` in {}", self.language);
            return id.to_owned();
        };
        let mut errors = Vec::new();
        let text = bundle.format_pattern(pattern, None, &mut errors);
        for error in errors {
            warn!("message `{id}`: {error}");
        }
        text.into_owned()
    }

    /// Puts the files together once they are all loaded, and again when one
    /// changes.
    fn assemble(
        mut events: MessageReader<AssetEvent<Messages>>,
        files: Res<Assets<Messages>>,
        mut locale: ResMut<Locale>,
    ) {
        let changed = events.read().any(|event| {
            matches!(
                event,
                AssetEvent::Added { .. } | AssetEvent::Modified { .. }
            )
        });
        if !changed {
            return;
        }
        let Some(loaded) = locale
            .files
            .iter()
            .map(|file| files.get(file))
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        let mut bundle = FluentBundle::new_concurrent(vec![locale.language.clone()]);
        // The fonts have no glyphs for the marks that keep right-to-left
        // text apart.
        bundle.set_use_isolating(false);
        for file in loaded {
            if let Err(errors) = bundle.add_resource(file.0.clone()) {
                for error in errors {
                    warn!("{}: {error}", locale.language);
                }
            }
        }
        locale.bundle = Some(bundle);
    }
}
