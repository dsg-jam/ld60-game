//! Text in the player's language, from the Fluent files in
//! `assets/locales/`, and the font that shows it.
//!
//! # Features
//!
//! - `test-support`: adds checks that read the files straight from disk.

#[cfg(any(test, feature = "test-support"))]
mod check;
mod localized;
mod messages;
mod typeface;

use std::borrow::Cow;
use std::sync::Arc;

use bevy::{prelude::*, text::detect_text_needs_rerender, ui::UiSystems};
use fluent_bundle::{
    FluentArgs, FluentError, FluentResource, FluentValue, concurrent::FluentBundle,
};
use unic_langid::{LanguageIdentifier, langid};

pub use self::localized::{Arg, Localized, Message};
use self::messages::{Messages, MessagesLoader};
pub use self::typeface::TypefacePlugin;

const LANGUAGE: LanguageIdentifier = langid!("en");
/// The Fluent files of each language, in `locales/<language>/`.
const FILES: &[&str] = &[
    "datasheet.ftl",
    "device.ftl",
    "editor.ftl",
    "levels.ftl",
    "manuals.ftl",
    "menu.ftl",
    "paper.ftl",
    "receipt.ftl",
    "rent.ftl",
    "salvage.ftl",
    "service.ftl",
    "ticket.ftl",
    "tutorial.ftl",
    "ui.ftl",
];

pub struct LocalePlugin;

impl Plugin for LocalePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Messages>()
            .init_asset_loader::<MessagesLoader>()
            .init_resource::<Locale>()
            .add_systems(Update, Locale::assemble)
            .add_systems(
                PostUpdate,
                Localized::show
                    .before(UiSystems::Content)
                    .before(detect_text_needs_rerender),
            );
    }
}

/// The messages of the player's language.
#[derive(Resource)]
pub struct Locale {
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
    /// The text in the player's language. A message is empty until the
    /// messages are loaded, and shows its ID if there is no such message.
    pub fn text(&self, localized: &Localized) -> String {
        match localized {
            Localized::Message(message) => self.message(message),
            Localized::Verbatim(text) => text.clone(),
        }
    }

    fn message(&self, message: &Message) -> String {
        let mut problems = Vec::new();
        let text = self.format(message, &mut problems);
        for problem in problems {
            warn!("{}: {problem}", self.language);
        }
        text
    }

    /// Formats `message` and adds what went wrong to `problems`.
    fn format(&self, message: &Message, problems: &mut Vec<String>) -> String {
        let Some(bundle) = &self.bundle else {
            return String::new();
        };
        let mut args = FluentArgs::new();
        for (name, arg) in &message.args {
            let value = match arg {
                Arg::Number(number) => FluentValue::from(*number),
                Arg::Text(text) => FluentValue::from(text.as_str()),
                Arg::Message(message) => FluentValue::from(self.format(message, problems)),
            };
            args.set(*name, value);
        }
        let mut errors = Vec::new();
        let Some(text) = Self::format_pattern(bundle, message, &args, &mut errors) else {
            problems.push(format!("no message `{}`", message.key()));
            return message.key();
        };
        for error in errors {
            problems.push(format!("message `{}`: {error}", message.key()));
        }
        text.into_owned()
    }

    /// Formats the message or attribute that `message` names, if there is
    /// one.
    fn format_pattern<'bundle>(
        bundle: &'bundle FluentBundle<Arc<FluentResource>>,
        message: &Message,
        args: &FluentArgs,
        errors: &mut Vec<FluentError>,
    ) -> Option<Cow<'bundle, str>> {
        let found = bundle.get_message(&message.id)?;
        let pattern = match &message.attribute {
            Some(attribute) => found.get_attribute(attribute)?.value(),
            None => found.value()?,
        };
        Some(bundle.format_pattern(pattern, Some(args), errors))
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
