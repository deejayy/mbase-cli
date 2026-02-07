use crate::error::{MbaseError, Result};
use crate::types::Mode;

pub mod confidence {
    pub const MULTIBASE_MATCH: f64 = 0.95;
    pub const ALPHABET_MATCH: f64 = 0.70;
    pub const PARTIAL_MATCH: f64 = 0.50;
    pub const WEAK_MATCH: f64 = 0.30;
}

pub fn clean_for_mode(input: &str, mode: Mode) -> String {
    match mode {
        Mode::Strict => input.to_string(),
        Mode::Lenient => input.chars().filter(|c| !c.is_ascii_whitespace()).collect(),
    }
}

pub fn validate_alphabet(input: &str, alphabet: &str, mode: Mode) -> Result<()> {
    let cleaned = clean_for_mode(input, mode);
    for (pos, ch) in cleaned.chars().enumerate() {
        if !alphabet.contains(ch) {
            return Err(MbaseError::InvalidCharacter { char: ch, position: pos });
        }
    }
    Ok(())
}

pub fn validate_alphabet_with_padding(input: &str, alphabet: &str, allow_padding: bool) -> Result<()> {
    for (pos, ch) in input.chars().enumerate() {
        if !alphabet.contains(ch) {
            if allow_padding && ch == '=' {
                continue;
            }
            return Err(MbaseError::InvalidCharacter { char: ch, position: pos });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_for_mode_strict() {
        assert_eq!(clean_for_mode("ab c\td", Mode::Strict), "ab c\td");
    }

    #[test]
    fn test_clean_for_mode_lenient() {
        assert_eq!(clean_for_mode("ab c\td\n", Mode::Lenient), "abcd");
    }

    #[test]
    fn test_validate_alphabet_success() {
        assert!(validate_alphabet("abc123", "abcdefghijklmnopqrstuvwxyz0123456789", Mode::Strict).is_ok());
    }

    #[test]
    fn test_validate_alphabet_invalid_char() {
        let result = validate_alphabet("abc!", "abc", Mode::Strict);
        assert!(result.is_err());
        match result {
            Err(MbaseError::InvalidCharacter { char: ch, position }) => {
                assert_eq!(ch, '!');
                assert_eq!(position, 3);
            }
            _ => panic!("expected InvalidCharacter error"),
        }
    }

    #[test]
    fn test_validate_alphabet_lenient_whitespace() {
        assert!(validate_alphabet("ab c\td", "abcd", Mode::Lenient).is_ok());
    }

    #[test]
    fn test_validate_alphabet_with_padding() {
        assert!(validate_alphabet_with_padding("SGVsbG8=", "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/", true).is_ok());
    }

    #[test]
    fn test_validate_alphabet_with_padding_reject_when_not_allowed() {
        let result = validate_alphabet_with_padding("SGVsbG8=", "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/", false);
        assert!(result.is_err());
    }
}
