use std::{fmt, io};

use fluent_bundle::{FluentArgs, FluentBundle, FluentError, FluentResource};
use ld60_levels::{ENGLISH_TEXT, Level};
use unic_langid::langid;

/// The English text of the levels, from the game's Fluent file.
pub struct LevelText {
    bundle: FluentBundle<FluentResource>,
}

#[derive(Debug)]
pub enum TextError {
    Read {
        path: &'static str,
        source: io::Error,
    },
    Parse {
        path: &'static str,
        errors: Vec<FluentError>,
    },
    Format {
        id: String,
        attribute: String,
        errors: Vec<FluentError>,
    },
}

impl LevelText {
    pub fn read() -> Result<Self, TextError> {
        let path = ENGLISH_TEXT;
        let source =
            std::fs::read_to_string(path).map_err(|source| TextError::Read { path, source })?;
        let resource = FluentResource::try_new(source).map_err(|(_, errors)| TextError::Parse {
            path,
            errors: errors.into_iter().map(FluentError::ParserError).collect(),
        })?;
        let mut bundle = FluentBundle::new(vec![langid!("en")]);
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .map_err(|errors| TextError::Parse { path, errors })?;
        Ok(Self { bundle })
    }

    /// The `attribute` of the message of `level`, or its ID if there is none.
    pub fn get(&self, level: &Level, attribute: &str) -> Result<String, TextError> {
        let Some(pattern) = self
            .bundle
            .get_message(&level.id)
            .and_then(|message| message.get_attribute(attribute))
            .map(|attribute| attribute.value())
        else {
            return Ok(format!("{}.{attribute}", level.id));
        };
        let mut args = FluentArgs::new();
        for (name, value) in level.text_args() {
            args.set(name, value);
        }
        let mut errors = Vec::new();
        let text = self
            .bundle
            .format_pattern(pattern, Some(&args), &mut errors)
            .into_owned();
        if errors.is_empty() {
            Ok(text)
        } else {
            Err(TextError::Format {
                id: level.id.clone(),
                attribute: attribute.to_owned(),
                errors,
            })
        }
    }
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let errors = match self {
            Self::Read { path, .. } => return write!(f, "cannot read {path}"),
            Self::Parse { path, errors } => {
                write!(f, "invalid {path}")?;
                errors
            }
            Self::Format {
                id,
                attribute,
                errors,
            } => {
                write!(f, "cannot format {id}.{attribute}")?;
                errors
            }
        };
        errors.iter().try_for_each(|e| write!(f, "\n  {e}"))
    }
}

impl std::error::Error for TextError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { .. } | Self::Format { .. } => None,
        }
    }
}
