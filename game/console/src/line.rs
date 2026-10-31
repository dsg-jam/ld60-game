use std::error::Error;
use std::fmt;

use ld60_game_core::debug_port::{DebugMethod, DebugMethodList, ParamKind};
use serde_json::{Map, Number, Value};

/// Methods without a namespace are in this one.
const NAMESPACE: &str = "ld60.";

/// A method call typed into the console, like `device open screw=2` or
/// `camera eye=1,2,3 at=0,1,0`.
///
/// The first word names the method, `ld60.` is implied. Then come
/// `name=value` pairs, the names of flags that are set, and values for the
/// other parameters in order. JSON after the name is passed on as is.
#[derive(Debug, PartialEq)]
pub(crate) struct Call {
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

#[derive(Debug)]
pub(crate) enum LineError {
    Empty,
    UnclosedQuote,
    Json(serde_json::Error),
    UnknownParam { method: String, param: String },
    ExtraWord(String),
    NotAFlag { param: String, value: String },
    NotNumbers { param: String, value: String },
    NotAChoice { param: String, value: String },
}

impl Call {
    pub(crate) fn parse(line: &str, methods: &DebugMethodList) -> Result<Self, LineError> {
        let line = line.trim();
        let (name, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        if name.is_empty() {
            return Err(LineError::Empty);
        }
        let method = Self::full_name(name);
        let rest = rest.trim_start();
        if rest.starts_with(['{', '[']) {
            let params = serde_json::from_str(rest).map_err(LineError::Json)?;
            return Ok(Self {
                method,
                params: Some(params),
            });
        }
        let spec = methods.get(&method);
        let mut params = Map::new();
        for word in words(rest)? {
            let (param, value) = match word.split_once('=') {
                Some((param, value)) => (param.to_owned(), value),
                None => match spec.and_then(|spec| Self::slot(spec, &word, &params)) {
                    Some((param, true)) => (param.to_owned(), "true"),
                    Some((param, false)) => (param.to_owned(), word.as_str()),
                    None => return Err(LineError::ExtraWord(word)),
                },
            };
            let kind = match spec {
                Some(spec) => Some(
                    spec.params()
                        .iter()
                        .find(|known| known.name == param)
                        .ok_or_else(|| LineError::UnknownParam {
                            method: method.clone(),
                            param: param.clone(),
                        })?
                        .kind,
                ),
                None => None,
            };
            let value = Self::value(&param, kind, value)?;
            params.insert(param, value);
        }
        Ok(Self {
            method,
            params: (!params.is_empty()).then_some(Value::Object(params)),
        })
    }

    /// `name` with the namespace, unless it has one.
    pub(crate) fn full_name(name: &str) -> String {
        if name.contains('.') {
            name.to_owned()
        } else {
            format!("{NAMESPACE}{name}")
        }
    }

    /// `name` without the implied namespace.
    pub(crate) fn short_name(name: &str) -> &str {
        name.strip_prefix(NAMESPACE).unwrap_or(name)
    }

    /// The parameter a bare `word` sets: a flag of that name, or else the
    /// next parameter that takes a value and is not set yet. `true` marks a
    /// flag.
    fn slot<'a>(
        spec: &'a DebugMethod,
        word: &str,
        set: &Map<String, Value>,
    ) -> Option<(&'a str, bool)> {
        let params = spec.params();
        if let Some(flag) = params
            .iter()
            .find(|param| param.kind == ParamKind::Flag && param.name == word)
        {
            return Some((flag.name, true));
        }
        params
            .iter()
            .find(|param| param.kind != ParamKind::Flag && !set.contains_key(param.name))
            .map(|param| (param.name, false))
    }

    fn value(param: &str, kind: Option<ParamKind>, value: &str) -> Result<Value, LineError> {
        let flag = || match value {
            "true" | "on" | "yes" => Some(Value::Bool(true)),
            "false" | "off" | "no" => Some(Value::Bool(false)),
            _ => None,
        };
        let numbers = || {
            let numbers = value
                .split(',')
                .map(|number| number.trim().parse::<Number>().map(Value::Number))
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            Some(match <[Value; 1]>::try_from(numbers) {
                Ok([number]) => number,
                Err(numbers) => Value::Array(numbers),
            })
        };
        let error = |make: fn(String, String) -> LineError| make(param.into(), value.into());
        match kind {
            Some(ParamKind::Flag) => {
                flag().ok_or_else(|| error(|param, value| LineError::NotAFlag { param, value }))
            }
            Some(ParamKind::Numbers) => numbers()
                .ok_or_else(|| error(|param, value| LineError::NotNumbers { param, value })),
            Some(ParamKind::Text) => Ok(Value::String(value.into())),
            Some(ParamKind::Choice(choices)) => choices
                .contains(&value)
                .then(|| Value::String(value.into()))
                .ok_or_else(|| error(|param, value| LineError::NotAChoice { param, value })),
            None => Ok(flag()
                .or_else(numbers)
                .unwrap_or_else(|| Value::String(value.into()))),
        }
    }
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "type a command, or help"),
            Self::UnclosedQuote => write!(f, "a quote is not closed"),
            Self::Json(_) => write!(f, "the parameters are not valid JSON"),
            Self::UnknownParam { method, param } => {
                write!(f, "{} has no parameter {param}", Call::short_name(method))
            }
            Self::ExtraWord(word) => write!(f, "nothing takes {word}"),
            Self::NotAFlag { param, value } => {
                write!(f, "{param} is true or false, not {value}")
            }
            Self::NotNumbers { param, value } => {
                write!(f, "{param} takes numbers like 1,2,3, not {value}")
            }
            Self::NotAChoice { param, value } => write!(f, "{param} cannot be {value}"),
        }
    }
}

impl Error for LineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

/// The words of `line`, split at spaces outside of quotes.
pub(crate) fn words(line: &str) -> Result<Vec<String>, LineError> {
    let mut words = Vec::new();
    let mut word = None::<String>;
    let mut quote = None;
    for c in line.chars() {
        match (quote, c) {
            (Some(open), c) if c == open => quote = None,
            (Some(_), c) => word.get_or_insert_default().push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                word.get_or_insert_default();
            }
            (None, c) if c.is_whitespace() => words.extend(word.take()),
            (None, c) => word.get_or_insert_default().push(c),
        }
    }
    if quote.is_some() {
        return Err(LineError::UnclosedQuote);
    }
    words.extend(word);
    Ok(words)
}

#[cfg(test)]
pub(crate) mod tests {
    use serde_json::json;

    use super::*;

    pub(crate) fn methods() -> DebugMethodList {
        [
            DebugMethod::new("ld60.device", "Works the devices.")
                .param("open", ParamKind::Flag, "opens the cover")
                .param("turn", ParamKind::Numbers, "")
                .param("screw", ParamKind::Numbers, ""),
            DebugMethod::new("ld60.editor", "").param(
                "history",
                ParamKind::Choice(&["undo", "redo"]),
                "",
            ),
        ]
        .into_iter()
        .collect()
    }

    fn parse(line: &str) -> Result<Call, LineError> {
        Call::parse(line, &methods())
    }

    #[test]
    fn flags_pairs_and_bare_values_become_params() {
        let call = parse("device open turn=0,180,0 2").unwrap();
        assert_eq!(call.method, "ld60.device");
        assert_eq!(
            call.params,
            Some(json!({ "open": true, "turn": [0, 180, 0], "screw": 2 }))
        );
        assert_eq!(
            parse("device open=false").unwrap().params,
            Some(json!({ "open": false }))
        );
        assert_eq!(parse("device").unwrap().params, None);
    }

    #[test]
    fn json_and_other_namespaces_pass_through() {
        let call = parse(r#"world.query {"data": {}}"#).unwrap();
        assert_eq!(call.method, "world.query");
        assert_eq!(call.params, Some(json!({ "data": {} })));
    }

    #[test]
    fn methods_without_a_description_guess_the_values() {
        let call = parse(r#"wind at=0,1,0 speed=2 name="a b" on=yes"#).unwrap();
        assert_eq!(
            call.params,
            Some(json!({ "at": [0, 1, 0], "speed": 2, "name": "a b", "on": true }))
        );
    }

    #[test]
    fn mistakes_are_named() {
        assert!(matches!(parse(""), Err(LineError::Empty)));
        assert!(matches!(
            parse("device \"open"),
            Err(LineError::UnclosedQuote)
        ));
        assert!(matches!(
            parse("device lid=1"),
            Err(LineError::UnknownParam { .. })
        ));
        assert!(matches!(
            parse("device open=maybe"),
            Err(LineError::NotAFlag { .. })
        ));
        assert!(matches!(
            parse("device turn=up"),
            Err(LineError::NotNumbers { .. })
        ));
        assert!(matches!(
            parse("editor sideways"),
            Err(LineError::NotAChoice { .. })
        ));
        assert!(matches!(
            parse("device 1,2,3 4 5"),
            Err(LineError::ExtraWord(_))
        ));
    }
}
