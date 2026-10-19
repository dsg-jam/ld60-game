use ld60_devices::{DeviceError, IsaKind, ProgramError};
use ld60_levels::LevelError;
use ld60_machine::AsmErrorKind;

use crate::device::{BoardLetter, DeviceName};
use crate::locale::Message;

impl From<&LevelError> for Message {
    /// The first assembler error with its line, or the error itself.
    fn from(err: &LevelError) -> Self {
        match err {
            LevelError::Assemble(errors) => match errors.as_slice() {
                [] => Self::new("editor-assembly-failed"),
                [only] => Self::new("editor-asm-error")
                    .with("line", only.line)
                    .with("message", Self::from(&only.kind)),
                [first, rest @ ..] => Self::new("editor-asm-error-more")
                    .with("line", first.line)
                    .with("message", Self::from(&first.kind))
                    .with("more", rest.len()),
            },
            LevelError::Parse(err) => Self::new("editor-error-parse")
                .with("line", err.span.start.line)
                .with("column", err.span.start.col),
            LevelError::Write(_) => Self::new("editor-error-write"),
            LevelError::BoardNotAllowed(rev) => Self::new("editor-error-board-not-allowed")
                .with("board", BoardLetter(*rev).to_string()),
            LevelError::NoRewrite(rev) => {
                Self::new("editor-error-no-rewrite").with("board", BoardLetter(*rev).to_string())
            }
            LevelError::Device(DeviceError::ChipMismatch { device, chip }) => {
                let chip = match chip {
                    IsaKind::Calco => "CalCo",
                };
                Self::new("editor-error-chip-mismatch")
                    .with("device", device.name())
                    .with("chip", chip)
            }
            LevelError::Device(DeviceError::RomSize { expected, actual }) => {
                Self::new("editor-error-rom-size")
                    .with("expected", *expected)
                    .with("actual", *actual)
            }
        }
    }
}

impl From<&AsmErrorKind> for Message {
    fn from(kind: &AsmErrorKind) -> Self {
        match kind {
            AsmErrorKind::InvalidLabel { name } => {
                Self::new("editor-asm-invalid-label").with("name", name.as_str())
            }
            AsmErrorKind::InvalidNumber { text } => {
                Self::new("editor-asm-invalid-number").with("text", text.as_str())
            }
            AsmErrorKind::InvalidOperand { text } => {
                Self::new("editor-asm-invalid-operand").with("text", text.as_str())
            }
            AsmErrorKind::DirectiveExpectsNumbers { directive } => {
                Self::new("editor-asm-directive-expects-numbers")
                    .with("directive", directive.as_str())
            }
            AsmErrorKind::OrgExpectsAddress => Self::new("editor-asm-org-expects-address"),
            AsmErrorKind::DbExpectsValue => Self::new("editor-asm-db-expects-value"),
            AsmErrorKind::UnknownDirective { directive } => {
                Self::new("editor-asm-unknown-directive").with("directive", directive.as_str())
            }
            AsmErrorKind::DuplicateLabel { name } => {
                Self::new("editor-asm-duplicate-label").with("name", name.as_str())
            }
            AsmErrorKind::OutsideRom { addr } => {
                Self::new("editor-asm-outside-rom").with("address", addr.to_string())
            }
            AsmErrorKind::RomFull => Self::new("editor-asm-rom-full"),
            AsmErrorKind::Overlap { addr } => {
                Self::new("editor-asm-overlap").with("address", format!("0x{addr:02X}"))
            }
            AsmErrorKind::WordRange { value } => {
                Self::new("editor-asm-word-range").with("value", value.to_string())
            }
            AsmErrorKind::UnknownLabel { name } => {
                Self::new("editor-asm-unknown-label").with("name", name.as_str())
            }
            AsmErrorKind::ExpectsValue => Self::new("editor-asm-expects-value"),
            AsmErrorKind::UnknownMnemonic { mnemonic } => {
                Self::new("editor-asm-unknown-mnemonic").with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::Unsupported { form, isa } => Self::new("editor-asm-unsupported")
                .with("form", form.as_str())
                .with("chip", *isa),
            AsmErrorKind::TakesNoOperands { mnemonic } => {
                Self::new("editor-asm-takes-no-operands").with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::ExpectsOneOperand { mnemonic } => {
                Self::new("editor-asm-expects-one-operand").with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::ExpectsRegisterAndValue { mnemonic } => {
                Self::new("editor-asm-expects-register-and-value")
                    .with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::ExpectsAOrB { mnemonic } => {
                Self::new("editor-asm-expects-a-or-b").with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::ExpectsMemory { mnemonic } => {
                Self::new("editor-asm-expects-memory").with("mnemonic", mnemonic.as_str())
            }
            AsmErrorKind::OffsetRange { offset } => {
                Self::new("editor-asm-offset-range").with("offset", offset.to_string())
            }
            AsmErrorKind::UnknownPort { name } => {
                Self::new("editor-asm-unknown-port").with("name", name.as_str())
            }
            AsmErrorKind::PortRange { port } => {
                Self::new("editor-asm-port-range").with("port", port.to_string())
            }
            AsmErrorKind::ExpectsPort => Self::new("editor-asm-expects-port"),
            AsmErrorKind::ByteRange { value } => {
                Self::new("editor-asm-byte-range").with("value", value.to_string())
            }
        }
    }
}

impl From<ProgramError> for Message {
    fn from(err: ProgramError) -> Self {
        match err {
            ProgramError::MaskRom => Self::new("editor-burn-mask-rom"),
            ProgramError::SizeMismatch => Self::new("editor-burn-size-mismatch"),
            ProgramError::BitConflict { addr } => {
                Self::new("editor-burn-bit-conflict").with("address", format!("0x{addr:02X}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_devices::{BoardRev, DeviceKind};
    use ld60_levels::Level;
    use ld60_machine::AsmError;

    use super::*;
    use crate::locale::Locale;

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
        Locale::on_disk().assert_formats(errors.map(|err| Message::from(&err)));
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
        locale.assert_formats(errors.iter().map(Message::from));
        locale.assert_formats(burns.map(Message::from));
    }

    #[test]
    fn parse_errors_show_where() {
        let err = Level::from_ron("(\n  id: 3,\n)").unwrap_err();
        let text = Locale::on_disk()
            .check(&Message::from(&err).into())
            .unwrap();
        assert!(text.contains("line 2"), "{text}");
    }
}
