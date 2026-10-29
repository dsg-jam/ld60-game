use std::sync::Arc;

use bevy::prelude::*;
use fluent_bundle::{FluentArgs, FluentResource};

use super::{FILES, LANGUAGE, Locale, Localized, Message};

impl Locale {
    /// The messages of the default language, read straight from the files.
    pub fn on_disk() -> Self {
        let dir = format!("{}/locales/{LANGUAGE}", env!("LD60_ASSETS"));
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

    /// The messages on disk, with the ones in `source` in their place.
    pub fn overridden(mut self, source: &str) -> Self {
        let resource = FluentResource::try_new(source.into()).unwrap();
        if let Some(bundle) = &mut self.bundle {
            bundle.add_resource_overriding(Arc::new(resource));
        }
        self
    }

    /// Whether the message or its attribute exists.
    pub fn has(&self, message: &Message) -> bool {
        self.bundle.as_ref().is_some_and(|bundle| {
            Self::format_pattern(bundle, message, &FluentArgs::new(), &mut Vec::new()).is_some()
        })
    }

    /// The text of `localized`, or what is wrong with it.
    pub fn check(&self, localized: &Localized) -> Result<String, Vec<String>> {
        let Localized::Message(message) = localized else {
            return Ok(self.text(localized));
        };
        let mut problems = Vec::new();
        let text = self.format(message, &mut problems);
        if problems.is_empty() {
            Ok(text)
        } else {
            Err(problems)
        }
    }

    /// Panics if one of `texts` names a missing message or leaves out an
    /// argument.
    pub fn assert_formats(&self, texts: impl IntoIterator<Item = impl Into<Localized>>) {
        let problems: Vec<_> = texts
            .into_iter()
            .filter_map(|text| self.check(&text.into()).err())
            .flatten()
            .collect();
        assert!(problems.is_empty(), "{problems:#?}");
    }
}

impl Localized {
    /// Every text that `write` spawns.
    pub fn spawned_by(write: impl FnOnce(&mut ChildSpawnerCommands)) -> Vec<Self> {
        let mut world = World::new();
        world.commands().spawn_empty().with_children(write);
        world.flush();
        world.query::<&Localized>().iter(&world).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_missing_messages_and_arguments() {
        let locale = Locale::on_disk();
        assert!(
            locale
                .check(&Message::new("no-such-message").into())
                .is_err()
        );
        assert!(
            locale
                .check(&Message::new("receipt-amount").into())
                .is_err()
        );
        let nested = Message::new("paper-quote").with("text", Message::new("no-such-message"));
        assert!(locale.check(&nested.into()).is_err());
        let amount = Message::new("receipt-amount").with("amount", 5_u32);
        assert_eq!(locale.check(&amount.into()), Ok("$5".into()));
    }
}
