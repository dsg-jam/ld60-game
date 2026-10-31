use bevy::prelude::*;
use ld60_game_core::debug_port::{DebugMethod, DebugMethodList, ParamKind};

use super::line::{Call, words};

/// Commands of the console itself, next to the methods.
pub(crate) const BUILTINS: [&str; 2] = ["help", "clear"];

/// What Tab does to the end of a line: it completes the last word as far
/// as all the options agree.
#[derive(Debug, PartialEq)]
pub(crate) struct Completion {
    pub(crate) line: String,
    /// What the word could become, when that is more than one thing.
    pub(crate) options: Vec<String>,
}

impl Completion {
    /// Completes the end of `line`. `list` finds the names of a
    /// [`ParamKind::Listed`] parameter.
    pub(crate) fn of(line: &str, methods: &DebugMethodList, list: &mut Lister) -> Self {
        let (head, partial) = line.split_at(last_word(line));
        let options: Vec<String> = Self::candidates(head, partial, methods, list)
            .into_iter()
            .filter(|option| unquoted(option).starts_with(&unquoted(partial)))
            .collect();
        let line = match options.as_slice() {
            [] => line.to_owned(),
            [only] => {
                let space = if only.ends_with('=') { "" } else { " " };
                format!("{head}{only}{space}")
            }
            [first, rest @ ..] => {
                let shared = rest.iter().fold(first.len(), |shared, option| {
                    first
                        .bytes()
                        .zip(option.bytes())
                        .take(shared)
                        .take_while(|(a, b)| a == b)
                        .count()
                });
                format!("{head}{}", &first[..shared])
            }
        };
        let options = if options.len() > 1 {
            options
        } else {
            Vec::new()
        };
        Self { line, options }
    }

    /// Every word that could stand after `head`, which ends in a space or
    /// is empty.
    fn candidates(
        head: &str,
        partial: &str,
        methods: &DebugMethodList,
        list: &mut Lister,
    ) -> Vec<String> {
        let Ok(before) = words(head) else {
            return Vec::new();
        };
        let names = || {
            methods
                .iter()
                .map(|method| Call::short_name(method.name()).to_owned())
        };
        let Some(first) = before.first() else {
            return BUILTINS
                .map(str::to_owned)
                .into_iter()
                .chain(names())
                .collect();
        };
        if first == "help" {
            return if before.len() == 1 {
                names().collect()
            } else {
                Vec::new()
            };
        }
        let Some(method) = methods.get(&Call::full_name(first)) else {
            return Vec::new();
        };
        if let Some((name, _)) = partial.split_once('=') {
            return method
                .params()
                .iter()
                .find(|param| param.name == name)
                .map(|param| Self::values(param.kind, list))
                .unwrap_or_default()
                .into_iter()
                .map(|value| format!("{name}={value}"))
                .collect();
        }
        let set = Self::set(method, &before[1..]);
        let next = method
            .params()
            .iter()
            .find(|param| !matches!(param.kind, ParamKind::Flag) && !set.contains(&param.name));
        let mut options: Vec<String> = method
            .params()
            .iter()
            .filter(|param| !set.contains(&param.name))
            .map(|param| match param.kind {
                ParamKind::Flag => param.name.to_owned(),
                _ => format!("{}=", param.name),
            })
            .collect();
        if let Some(next) = next {
            options.extend(Self::values(next.kind, list));
        }
        options
    }

    /// The parameters of `method` that `words` set, as [`Call::parse`]
    /// reads them.
    fn set(method: &DebugMethod, words: &[String]) -> Vec<&'static str> {
        let mut set = Vec::new();
        for word in words {
            let name = match word.split_once('=') {
                Some((name, _)) => method
                    .params()
                    .iter()
                    .find(|param| param.name == name)
                    .map(|param| param.name),
                None => method
                    .params()
                    .iter()
                    .find(|param| matches!(param.kind, ParamKind::Flag) && param.name == word)
                    .or_else(|| {
                        method.params().iter().find(|param| {
                            !matches!(param.kind, ParamKind::Flag) && !set.contains(&param.name)
                        })
                    })
                    .map(|param| param.name),
            };
            set.extend(name);
        }
        set
    }

    /// The values a parameter of `kind` can have, quoted where they have
    /// spaces.
    fn values(kind: ParamKind, list: &mut Lister) -> Vec<String> {
        let values = match kind {
            ParamKind::Choice(choices) => choices.iter().map(|&choice| choice.to_owned()).collect(),
            ParamKind::Flag => vec!["true".to_owned(), "false".to_owned()],
            ParamKind::Listed(names) => list(names),
            ParamKind::Numbers | ParamKind::Text => Vec::new(),
        };
        values
            .into_iter()
            .map(|value| {
                if value.contains(char::is_whitespace) {
                    format!("\"{value}\"")
                } else {
                    value
                }
            })
            .collect()
    }
}

/// Finds the names of a [`ParamKind::Listed`] parameter with its function.
pub(crate) type Lister<'a> = dyn FnMut(fn(&mut World) -> Vec<String>) -> Vec<String> + 'a;

/// Where the last word of `line` starts: after the last space outside of
/// quotes.
fn last_word(line: &str) -> usize {
    let mut quote = None;
    let mut start = 0;
    for (index, c) in line.char_indices() {
        match (quote, c) {
            (Some(open), c) if c == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, c) if c.is_whitespace() => start = index + c.len_utf8(),
            (None, _) => {}
        }
    }
    start
}

/// `word` without its quotes.
fn unquoted(word: &str) -> String {
    word.replace('"', "")
}

#[cfg(test)]
mod tests {
    use super::super::line::tests::methods;
    use super::*;

    fn complete(line: &str) -> Completion {
        Completion::of(line, &methods(), &mut |_| {
            vec![
                "shop manuals".to_owned(),
                "shop sign".to_owned(),
                "device".to_owned(),
            ]
        })
    }

    #[test]
    fn a_single_option_is_filled_in() {
        assert_eq!(complete("dev").line, "device ");
        assert_eq!(complete("device op").line, "device open ");
        assert_eq!(complete("device t").line, "device turn=");
        assert_eq!(complete("editor history=u").line, "editor history=undo ");
        assert_eq!(complete("help ed").line, "help editor ");
    }

    #[test]
    fn several_options_are_listed_and_filled_in_as_far_as_they_agree() {
        let completion = complete("device ");
        assert_eq!(completion.line, "device ");
        assert_eq!(completion.options, ["open", "turn=", "screw="]);
        assert_eq!(
            complete("").options,
            ["help", "clear", "device", "editor", "focus"]
        );
    }

    #[test]
    fn names_from_the_world_are_offered_and_quoted() {
        assert_eq!(complete("focus d").line, "focus device ");
        assert_eq!(complete("focus target=sh").line, "focus target=\"shop ");
        assert_eq!(
            complete("focus target=\"shop m").line,
            "focus target=\"shop manuals\" "
        );
        assert_eq!(complete("focus shop m").line, "focus shop m");
    }

    #[test]
    fn params_already_set_are_not_offered_again() {
        assert_eq!(
            complete("device open turn=1 ").line,
            "device open turn=1 screw="
        );
    }
}
