/// A byte value that keeps the spelling it was written in, like `0x0A` or `10`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Literal {
    pub(crate) value: u8,
    pub(crate) text: String,
}

impl Literal {
    pub(crate) fn decimal(value: u8) -> Self {
        Self {
            value,
            text: value.to_string(),
        }
    }

    pub(crate) fn hex(value: u8) -> Self {
        Self {
            value,
            text: format!("0x{value:02X}"),
        }
    }

    /// A new value written in the same radix as `self`.
    pub(crate) fn with_value(&self, value: u8) -> Self {
        if self.is_hex() {
            Self::hex(value)
        } else {
            Self::decimal(value)
        }
    }

    fn is_hex(&self) -> bool {
        self.text.starts_with("0x") || self.text.starts_with("0X")
    }
}
