use std::borrow::Cow;

use bevy::prelude::*;

use super::Locale;

macro_rules! number_args {
    ($($number:ty),*) => {$(
        impl From<$number> for Arg {
            fn from(number: $number) -> Self {
                Self::Number(f64::from(number))
            }
        }
    )*};
}

/// Fills the [`Text`] of its entity with text in the player's language.
#[derive(Component, Clone, Debug, PartialEq)]
#[require(Text)]
pub(crate) enum Localized {
    Message(Message),
    /// Reads the same in every language, like a name or a serial number.
    Verbatim(String),
}

/// A Fluent message, or one of its attributes, and the values to fill into
/// it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Message {
    pub(super) id: Cow<'static, str>,
    pub(super) attribute: Option<Cow<'static, str>>,
    pub(super) args: Vec<(&'static str, Arg)>,
}

/// A value to fill into a [`Message`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Arg {
    Number(f64),
    /// Reads the same in every language.
    Text(String),
    /// Another message, in the player's language.
    Message(Message),
}

impl Localized {
    pub(crate) fn verbatim(text: impl Into<String>) -> Self {
        Self::Verbatim(text.into())
    }

    /// Formats every text when the messages change, otherwise only the
    /// changed ones.
    pub(super) fn show(
        locale: Res<Locale>,
        mut texts: Query<(&Localized, &mut Text)>,
        changed: Query<Entity, Changed<Localized>>,
    ) {
        if locale.is_changed() {
            for (localized, mut text) in &mut texts {
                text.0 = locale.text(localized);
            }
        } else {
            for entity in &changed {
                if let Ok((localized, mut text)) = texts.get_mut(entity) {
                    text.0 = locale.text(localized);
                }
            }
        }
    }
}

impl From<Message> for Localized {
    fn from(message: Message) -> Self {
        Self::Message(message)
    }
}

impl Message {
    pub(crate) fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self {
            id: id.into(),
            attribute: None,
            args: Vec::new(),
        }
    }

    pub(crate) fn attribute(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.attribute = Some(name.into());
        self
    }

    pub(crate) fn with(mut self, name: &'static str, arg: impl Into<Arg>) -> Self {
        self.args.push((name, arg.into()));
        self
    }

    /// The ID, with the attribute after a dot.
    pub(super) fn key(&self) -> String {
        match &self.attribute {
            Some(attribute) => format!("{}.{attribute}", self.id),
            None => self.id.to_string(),
        }
    }
}

impl From<Localized> for Arg {
    fn from(localized: Localized) -> Self {
        match localized {
            Localized::Message(message) => Self::Message(message),
            Localized::Verbatim(text) => Self::Text(text),
        }
    }
}

impl From<Message> for Arg {
    fn from(message: Message) -> Self {
        Self::Message(message)
    }
}

impl From<String> for Arg {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for Arg {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

number_args!(u8, u16, u32, i32);

impl From<usize> for Arg {
    #[expect(clippy::cast_precision_loss, reason = "counts stay small")]
    fn from(number: usize) -> Self {
        Self::Number(number as f64)
    }
}
