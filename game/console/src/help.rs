use std::fmt;

use ld60_game_core::debug_port::{DebugMethod, DebugMethodList, ParamKind};

use super::line::Call;

/// The text of `help`: every command, or one method and its parameters.
pub(crate) enum Help<'a> {
    All(&'a DebugMethodList),
    Method(&'a DebugMethod),
}

impl fmt::Display for Help<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All(methods) => {
                writeln!(f, "Tab completes. help NAME explains a command.")?;
                writeln!(f, "  {:<12} lists the commands", "help")?;
                write!(f, "  {:<12} empties the output", "clear")?;
                for method in methods.iter() {
                    write!(
                        f,
                        "\n  {:<12} {}",
                        Call::short_name(method.name()),
                        method.summary()
                    )?;
                }
                Ok(())
            }
            Self::Method(method) => {
                write!(
                    f,
                    "{}: {}",
                    Call::short_name(method.name()),
                    method.summary()
                )?;
                for param in method.params() {
                    let usage = match param.kind {
                        ParamKind::Flag => format!("{} | {}=false", param.name, param.name),
                        ParamKind::Numbers => format!("{}=1,2,3", param.name),
                        ParamKind::Text => format!("{}=\"...\"", param.name),
                        ParamKind::Choice(choices) => {
                            format!("{}={}", param.name, choices.join("|"))
                        }
                    };
                    write!(f, "\n  {usage:<28} {}", param.help)?;
                }
                Ok(())
            }
        }
    }
}
