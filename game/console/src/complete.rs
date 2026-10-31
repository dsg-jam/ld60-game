use ld60_game_core::debug_port::{DebugMethodList, ParamKind};

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
    pub(crate) fn of(line: &str, methods: &DebugMethodList) -> Self {
        let partial = line.rsplit(char::is_whitespace).next().unwrap_or_default();
        let head = &line[..line.len() - partial.len()];
        let options: Vec<String> = Self::candidates(head, partial, methods)
            .into_iter()
            .filter(|option| option.starts_with(partial))
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
    fn candidates(head: &str, partial: &str, methods: &DebugMethodList) -> Vec<String> {
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
            let values: &[&str] = match method.params().iter().find(|param| param.name == name) {
                Some(param) => match param.kind {
                    ParamKind::Choice(choices) => choices,
                    ParamKind::Flag => &["true", "false"],
                    ParamKind::Numbers | ParamKind::Text => &[],
                },
                None => &[],
            };
            return values
                .iter()
                .map(|value| format!("{name}={value}"))
                .collect();
        }
        let set: Vec<&str> = before[1..]
            .iter()
            .map(|word| word.split_once('=').map_or(word.as_str(), |(name, _)| name))
            .collect();
        method
            .params()
            .iter()
            .filter(|param| !set.contains(&param.name))
            .map(|param| match param.kind {
                ParamKind::Flag => param.name.to_owned(),
                _ => format!("{}=", param.name),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::line::tests::methods;
    use super::*;

    fn complete(line: &str) -> Completion {
        Completion::of(line, &methods())
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
        assert_eq!(complete("").options, ["help", "clear", "device", "editor"]);
    }

    #[test]
    fn params_already_set_are_not_offered_again() {
        assert_eq!(
            complete("device open turn=1 ").line,
            "device open turn=1 screw="
        );
    }
}
