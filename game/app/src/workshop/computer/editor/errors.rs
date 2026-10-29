use ld60_devices::{DeviceError, IsaKind, ProgramError};
use ld60_levels::LevelError;
use ld60_locale::Message;
use ld60_machine::AsmErrorKind;

use ld60_game_device::{BoardLetter, DeviceName};

/// What the editor says about an error.
pub(super) trait Explain {
    fn message(&self) -> Message;
}

impl Explain for LevelError {
    /// The first assembler error with its line, or the error itself.
    fn message(&self) -> Message {
        match self {
            LevelError::Assemble(errors) => match errors.as_slice() {
                [] => Message::new("editor-assembly-failed"),
                [only] => Message::new("editor-asm-error")
                    .with("line", only.line)
                    .with("message", only.kind.message()),
                [first, rest @ ..] => Message::new("editor-asm-error-more")
                    .with("line", first.line)
                    .with("message", first.kind.message())
                    .with("more", rest.len()),
            },
            LevelError::Parse(err) => Message::new("editor-error-parse")
                .with("line", err.span.start.line)
                .with("column", err.span.start.col),
            LevelError::Write(_) => Message::new("editor-error-write"),
            LevelError::BoardNotAllowed(rev) => Message::new("editor-error-board-not-allowed")
                .with("board", BoardLetter(*rev).to_string()),
            LevelError::NoRewrite(rev) => {
                Message::new("editor-error-no-rewrite").with("board", BoardLetter(*rev).to_string())
            }
            LevelError::Device(DeviceError::ChipMismatch { device, chip }) => {
                let chip = match chip {
                    IsaKind::Calco => "CalCo",
                };
                Message::new("editor-error-chip-mismatch")
                    .with("device", device.name())
                    .with("chip", chip)
            }
            LevelError::Device(DeviceError::RomSize { expected, actual }) => {
                Message::new("editor-error-rom-size")
                    .with("expected", *expected)
                    .with("actual", *actual)
            }
        }
    }
}

impl Explain for AsmErrorKind {
    fn message(&self) -> Message {
        match self {
            AsmErrorKind::InvalidLabel { name } => {
                Message::new("editor-asm-invalid-label").with("name", name.as_str())
            }
            AsmErrorKind::InvalidNumber { text } => {
                Message::new("editor-asm-invalid-number").with("text", text.as_str())
            }
            AsmErrorKind::InvalidOperand { text } => {
                Message::new("editor-asm-invalid-operand").with("text", text.as_str())
            }
            AsmErrorKind::DirectiveExpectsNumbers { directive } => {
                Message::new("editor-asm-directive-expects-numbers")
                    .with("directive", directive.as_str())
            }
            AsmErrorKind::OrgExpectsAddress => Message::new("editor-asm-org-expects-address"),
            AsmErrorKind::DbExpectsValue => Message::new("editor-asm-db-expects-value"),
            AsmErrorKind::UnknownDirective { directive } => {
                Message::new("editor-asm-unknown-directive").with("directive", directive.as_str())
            }
            AsmErrorKind::DuplicateLabel { name } => {
                Message::new("editor-asm-duplicate-label").with("name", name.as_str())
            }
            AsmErrorKind::OutsideRom { addr } => {
                Message::new("editor-asm-outside-rom").with("address", addr.to_string())
            }
            AsmErrorKind::RomFull => Message::new("editor-asm-rom-full"),
            AsmErrorKind::Overlap { addr } => {
                Message::new("editor-asm-overlap").with("address", format!("0x{addr:02X}"))
            }
            AsmErrorKind::WordRange { value } => {
                Message::new("editor-asm-word-range").with("value", value.to_string())
            }
            AsmErrorKind::UnknownLabel { name } => {
                Message::new("editor-asm-unknown-label").with("name", name.as_str())
            }
            AsmErrorKind::ExpectsValue => Message::new("editor-asm-expects-value"),
            AsmErrorKind::UnknownMnemonic { mnemonic } => {
                Message::new("editor-asm-unknown-mnemonic").with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::Unsupported { form, isa } => Message::new("editor-asm-unsupported")
                .with("form", form.as_str())
                .with("chip", *isa),
            AsmErrorKind::TakesNoOperands { mnemonic } => {
                Message::new("editor-asm-takes-no-operands").with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::ExpectsOneOperand { mnemonic } => {
                Message::new("editor-asm-expects-one-operand").with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::ExpectsRegisterAndValue { mnemonic } => {
                Message::new("editor-asm-expects-register-and-value")
                    .with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::ExpectsAOrB { mnemonic } => {
                Message::new("editor-asm-expects-a-or-b").with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::ExpectsMemory { mnemonic } => {
                Message::new("editor-asm-expects-memory").with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::OffsetRange { offset } => {
                Message::new("editor-asm-offset-range").with("offset", offset.to_string())
            }
            AsmErrorKind::UnknownPort { name } => {
                Message::new("editor-asm-unknown-port").with("name", name.as_str())
            }
            AsmErrorKind::PortRange { port } => {
                Message::new("editor-asm-port-range").with("port", port.to_string())
            }
            AsmErrorKind::ExpectsPort => Message::new("editor-asm-expects-port"),
            AsmErrorKind::ByteRange { value } => {
                Message::new("editor-asm-byte-range").with("value", value.to_string())
            }
        }
    }
}

impl Explain for ProgramError {
    fn message(&self) -> Message {
        match self {
            ProgramError::MaskRom => Message::new("editor-burn-mask-rom"),
            ProgramError::SizeMismatch => Message::new("editor-burn-size-mismatch"),
            ProgramError::BitConflict { addr } => {
                Message::new("editor-burn-bit-conflict").with("address", format!("0x{addr:02X}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_devices::{BoardRev, DeviceKind};
    use ld60_levels::Level;
    use ld60_locale::Locale;
    use ld60_machine::AsmError;

    use super::*;

    fn kinds() -> Vec<AsmErrorKind> {
        let name = || "loop".to_string();
        let mnemonic = || "LDI".to_string();
        vec![
            AsmErrorKind::InvalidLabel { name: name() },
            AsmErrorKind::InvalidNumber { text: "12a".into() },
            AsmErrorKind::InvalidOperand {
                text: "[8+B]".into(),
            },
            AsmErrorKind::DirectiveExpectsNumbers {
                directive: "db".into(),
            },
            AsmErrorKind::OrgExpectsAddress,
            AsmErrorKind::DbExpectsValue,
            AsmErrorKind::UnknownDirective {
                directive: "fill".into(),
            },
            AsmErrorKind::DuplicateLabel { name: name() },
            AsmErrorKind::OutsideRom { addr: 300 },
            AsmErrorKind::RomFull,
            AsmErrorKind::Overlap { addr: 0x10 },
            AsmErrorKind::WordRange { value: 256 },
            AsmErrorKind::UnknownLabel { name: name() },
            AsmErrorKind::ExpectsValue,
            AsmErrorKind::UnknownMnemonic {
                mnemonic: "FOO".into(),
            },
            AsmErrorKind::Unsupported {
                form: "ADC M1".into(),
                isa: "CalCo Rev A",
            },
            AsmErrorKind::TakesNoOperands {
                mnemonic: mnemonic(),
            },
            AsmErrorKind::ExpectsOneOperand {
                mnemonic: mnemonic(),
            },
            AsmErrorKind::ExpectsRegisterAndValue {
                mnemonic: mnemonic(),
            },
            AsmErrorKind::ExpectsAOrB {
                mnemonic: mnemonic(),
            },
            AsmErrorKind::ExpectsMemory {
                mnemonic: mnemonic(),
            },
            AsmErrorKind::OffsetRange { offset: 16 },
            AsmErrorKind::UnknownPort { name: name() },
            AsmErrorKind::PortRange { port: 16 },
            AsmErrorKind::ExpectsPort,
            AsmErrorKind::ByteRange { value: -1 },
        ]
    }

    #[test]
    fn every_asm_error_formats() {
        let errors = kinds().into_iter().map(|kind| {
            LevelError::Assemble(vec![AsmError {
                line: 1,
                span: 0..1,
                kind,
            }])
        });
        Locale::on_disk().assert_formats(errors.map(|err| err.message()));
    }

    #[test]
    fn every_error_formats() {
        let asm = |line| AsmError {
            line,
            span: 0..1,
            kind: AsmErrorKind::RomFull,
        };
        let errors = [
            LevelError::Assemble(Vec::new()),
            LevelError::Assemble(vec![asm(1)]),
            LevelError::Assemble(vec![asm(1), asm(2)]),
            Level::from_ron("(").unwrap_err(),
            LevelError::Write(ron::Error::Eof),
            LevelError::BoardNotAllowed(BoardRev::B),
            LevelError::NoRewrite(BoardRev::C),
            LevelError::Device(DeviceError::ChipMismatch {
                device: DeviceKind::Summa6,
                chip: IsaKind::Calco,
            }),
            LevelError::Device(DeviceError::RomSize {
                expected: 256,
                actual: 1,
            }),
        ];
        let burns = [
            ProgramError::MaskRom,
            ProgramError::SizeMismatch,
            ProgramError::BitConflict { addr: 0x1F },
        ];
        let locale = Locale::on_disk();
        locale.assert_formats(errors.iter().map(Explain::message));
        locale.assert_formats(burns.iter().map(Explain::message));
    }

    #[test]
    fn parse_errors_show_where() {
        let err = Level::from_ron("(\n  id: 3,\n)").unwrap_err();
        let text = Locale::on_disk().check(&err.message().into()).unwrap();
        assert!(text.contains("line 2"), "{text}");
    }
}
