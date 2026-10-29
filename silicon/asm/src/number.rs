use ld60_machine::AsmErrorKind;

pub fn parse_number(text: &str) -> Result<i64, AsmErrorKind> {
    let invalid = || AsmErrorKind::InvalidNumber {
        text: text.to_string(),
    };
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (radix, digits) = match body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        Some(hex) => (16, hex),
        None => (10, body),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return Err(invalid());
    }
    let value = i64::from_str_radix(digits, radix).map_err(|_| invalid())?;
    Ok(if negative { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::parse_number;

    #[test]
    fn parses_decimal_and_hex() {
        assert_eq!(parse_number("42"), Ok(42));
        assert_eq!(parse_number("0x2A"), Ok(42));
        assert_eq!(parse_number("-1"), Ok(-1));
    }

    #[test]
    fn rejects_malformed_numbers() {
        for text in ["", "0x", "12a", "+5", "0x-5", "--1"] {
            assert!(parse_number(text).is_err(), "{text}");
        }
    }
}
