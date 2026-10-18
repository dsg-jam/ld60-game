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

/// A Fluent message and the values to fill into it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Message {
    pub(super) id: &'static str,
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

    pub(super) fn show(locale: Res<Locale>, mut texts: Query<(Ref<Localized>, &mut Text)>) {
        for (localized, mut text) in &mut texts {
            if locale.is_changed() || localized.is_changed() {
                text.0 = locale.text(&localized);
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
    pub(crate) const fn new(id: &'static str) -> Self {
        Self {
            id,
            args: Vec::new(),
        }
    }

    pub(crate) fn with(mut self, name: &'static str, arg: impl Into<Arg>) -> Self {
        self.args.push((name, arg.into()));
        self
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
