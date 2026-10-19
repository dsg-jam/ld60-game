use std::fmt;
use std::ops::Range;

/// An assembler error and where in the source it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AsmError {
    /// 1-based source line.
    pub line: usize,
    /// Byte range within the line that the error is about.
    pub span: Range<usize>,
    pub kind: AsmErrorKind,
}

impl fmt::Display for AsmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.kind)
    }
}

impl std::error::Error for AsmError {}

/// What is wrong with a line of assembly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AsmErrorKind {
    InvalidLabel {
        name: String,
    },
    InvalidNumber {
        text: String,
    },
    InvalidOperand {
        text: String,
    },
    /// The directive name is without its dot.
    DirectiveExpectsNumbers {
        directive: String,
    },
    OrgExpectsAddress,
    DbExpectsValue,
    /// The directive name is without its dot.
    UnknownDirective {
        directive: String,
    },
    DuplicateLabel {
        name: String,
    },
    OutsideRom {
        addr: i64,
    },
    RomFull,
    Overlap {
        addr: usize,
    },
    WordRange {
        value: i64,
    },
    UnknownLabel {
        name: String,
    },
    ExpectsValue,
    UnknownMnemonic {
        mnemonic: String,
    },
    /// The ISA knows the instruction, but this chip does not run it.
    Unsupported {
        /// The instruction as written in the datasheet, like `LD [B+2]`.
        form: String,
        /// [`Isa::name`](crate::Isa::name) of the chip.
        isa: &'static str,
    },
    TakesNoOperands {
        mnemonic: String,
    },
    ExpectsOneOperand {
        mnemonic: String,
    },
    ExpectsRegisterAndValue {
        mnemonic: String,
    },
    ExpectsAOrB {
        mnemonic: String,
    },
    ExpectsMemory {
        mnemonic: String,
    },
    OffsetRange {
        offset: i64,
    },
    UnknownPort {
        name: String,
    },
    PortRange {
        port: i64,
    },
    ExpectsPort,
    ByteRange {
        value: i64,
    },
}

impl fmt::Display for AsmErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLabel { name } => write!(f, "invalid label '{name}'"),
            Self::InvalidNumber { text } => write!(f, "invalid number '{text}'"),
            Self::InvalidOperand { text } => write!(f, "invalid operand '{text}'"),
            Self::DirectiveExpectsNumbers { directive } => {
                write!(f, ".{directive} expects numbers")
            }
            Self::OrgExpectsAddress => f.write_str(".org expects one address"),
            Self::DbExpectsValue => f.write_str(".db expects at least one value"),
            Self::UnknownDirective { directive } => write!(f, "unknown directive '.{directive}'"),
            Self::DuplicateLabel { name } => write!(f, "duplicate label '{name}'"),
            Self::OutsideRom { addr } => write!(f, "address {addr} is outside the ROM"),
            Self::RomFull => f.write_str("code does not fit in the ROM"),
            Self::Overlap { addr } => write!(f, "overlaps code at 0x{addr:02X}"),
            Self::WordRange { value } => write!(f, "value {value} does not fit in a word"),
            Self::UnknownLabel { name } => write!(f, "unknown label '{name}'"),
            Self::ExpectsValue => f.write_str("expected a number or label"),
            Self::UnknownMnemonic { mnemonic } => write!(f, "unknown mnemonic '{mnemonic}'"),
            Self::Unsupported { form, isa } => write!(f, "'{form}' is not supported by {isa}"),
            Self::TakesNoOperands { mnemonic } => write!(f, "{mnemonic} takes no operands"),
            Self::ExpectsOneOperand { mnemonic } => write!(f, "{mnemonic} expects one operand"),
            Self::ExpectsRegisterAndValue { mnemonic } => {
                write!(f, "{mnemonic} expects a register and a value")
            }
            Self::ExpectsAOrB { mnemonic } => write!(f, "{mnemonic} expects register A or B"),
            Self::ExpectsMemory { mnemonic } => write!(f, "{mnemonic} expects Mn or [B+n]"),
            Self::OffsetRange { .. } => f.write_str("offset must be 0 to 15"),
            Self::UnknownPort { name } => write!(f, "unknown port '{name}'"),
            Self::PortRange { .. } => f.write_str("port must be 0 to 15"),
            Self::ExpectsPort => f.write_str("expected a port"),
            Self::ByteRange { value } => write!(f, "value {value} is outside 0 to 255"),
        }
    }
}
