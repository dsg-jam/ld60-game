mod localized;
mod messages;

use std::sync::Arc;

use bevy::prelude::*;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use unic_langid::{LanguageIdentifier, langid};

pub(crate) use self::localized::{Arg, Localized, Message};
use self::messages::{Messages, MessagesLoader};

const LANGUAGE: LanguageIdentifier = langid!("en");
/// The Fluent files of each language, in `locales/<language>/`.
const FILES: &[&str] = &[
    "datasheet.ftl",
    "device.ftl",
    "editor.ftl",
    "levels.ftl",
    "paper.ftl",
    "receipt.ftl",
    "service.ftl",
    "ticket.ftl",
    "ui.ftl",
];

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

#[cfg(test)]
impl Locale {
    /// The messages of the default language, read straight from the files.
    pub(crate) fn on_disk() -> Self {
        let dir = format!("{}/assets/locales/{LANGUAGE}", env!("CARGO_MANIFEST_DIR"));
        let files = FILES.iter().map(|file| {
            let source = std::fs::read_to_string(format!("{dir}/{file}")).unwrap();
            Arc::new(FluentResource::try_new(source).unwrap())
        });
        Self {
            language: LANGUAGE,
            files: Vec::new(),
            bundle: Some(Self::bundle(&LANGUAGE, files)),
        }
    }

    /// Whether the message or its attribute exists.
    pub(crate) fn has(&self, message: &Message) -> bool {
        let found = self
            .bundle
            .as_ref()
            .and_then(|bundle| bundle.get_message(&message.id));
        match &message.attribute {
            Some(attribute) => found.is_some_and(|found| found.get_attribute(attribute).is_some()),
            None => found.is_some_and(|found| found.value().is_some()),
        }
    }
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
    /// The text in the player's language. A message is empty until the
    /// messages are loaded, and shows its ID if there is no such message.
    pub(crate) fn text(&self, localized: &Localized) -> String {
        match localized {
            Localized::Message(message) => self.message(message),
            Localized::Verbatim(text) => text.clone(),
        }
    }

    fn message(&self, message: &Message) -> String {
        let Some(bundle) = &self.bundle else {
            return String::new();
        };
        let id = match &message.attribute {
            Some(attribute) => format!("{}.{attribute}", message.id),
            None => message.id.to_string(),
        };
        let found = bundle.get_message(&message.id);
        let pattern = match &message.attribute {
            Some(attribute) => found
                .and_then(|found| found.get_attribute(attribute))
                .map(|attribute| attribute.value()),
            None => found.and_then(|found| found.value()),
        };
        let Some(pattern) = pattern else {
            warn!("no message `{id}` in {}", self.language);
            return id;
        };
        let mut args = FluentArgs::new();
        for (name, arg) in &message.args {
            let value = match arg {
                Arg::Number(number) => FluentValue::from(*number),
                Arg::Text(text) => FluentValue::from(text.as_str()),
                Arg::Message(message) => FluentValue::from(self.message(message)),
            };
            args.set(*name, value);
        }
        let mut errors = Vec::new();
        let text = bundle.format_pattern(pattern, Some(&args), &mut errors);
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
        let bundle = Self::bundle(&locale.language, loaded.iter().map(|file| file.0.clone()));
        locale.bundle = Some(bundle);
    }

    fn bundle(
        language: &LanguageIdentifier,
        files: impl IntoIterator<Item = Arc<FluentResource>>,
    ) -> FluentBundle<Arc<FluentResource>> {
        let mut bundle = FluentBundle::new_concurrent(vec![language.clone()]);
        // The fonts have no glyphs for the marks that keep right-to-left
        // text apart.
        bundle.set_use_isolating(false);
        for file in files {
            if let Err(errors) = bundle.add_resource(file) {
                for error in errors {
                    warn!("{language}: {error}");
                }
            }
        }
        bundle
    }
}
