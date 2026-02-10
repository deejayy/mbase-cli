use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct UrlEncoding;

impl Codec for UrlEncoding {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "urlencoding",
            aliases: &["url", "percent", "percentencoding"],
            alphabet: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~%",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "URL percent-encoding (RFC 3986)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let mut result = String::new();
        for &byte in input {
            let c = byte as char;
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                result.push(c);
            } else {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
        Ok(result)
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);
        let mut result = Vec::new();
        let mut chars = cleaned.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '%' {
                let hex1 = chars
                    .next()
                    .ok_or_else(|| MbaseError::invalid_input("incomplete percent sequence"))?;
                let hex2 = chars
                    .next()
                    .ok_or_else(|| MbaseError::invalid_input("incomplete percent sequence"))?;

                let hex_str = format!("{}{}", hex1, hex2);
                let byte = u8::from_str_radix(&hex_str, 16)
                    .map_err(|_| MbaseError::invalid_input(format!("invalid hex in percent sequence: {}", hex_str)))?;
                result.push(byte);
            } else if c.is_ascii() {
                result.push(c as u8);
            } else {
                return Err(MbaseError::invalid_input(format!("non-ASCII character in URL encoding: {}", c)));
            }
        }

        Ok(result)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "urlencoding".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let percent_count = input.matches('%').count();

        if percent_count > 0 {
            let valid_sequences = input
                .split('%')
                .skip(1)
                .filter(|s| s.len() >= 2 && s.chars().take(2).all(|c| c.is_ascii_hexdigit()))
                .count();

            if valid_sequences == percent_count {
                confidence = util::confidence::ALPHABET_MATCH;
                reasons.push(format!("found {} valid percent-encoded sequences", percent_count));
            } else if valid_sequences > 0 {
                confidence = util::confidence::WEAK_MATCH;
                reasons.push(format!("found {} valid sequences out of {} percent signs", valid_sequences, percent_count));
                warnings.push("some percent sequences appear invalid".to_string());
            }
        } else {
            let url_safe_count = input
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~' | '/' | '?' | '=' | '&'))
                .count();

            if url_safe_count as f64 / input.len() as f64 > 0.9 {
                confidence = 0.3;
                reasons.push("contains URL-safe characters but no encoding".to_string());
                warnings.push("could be plain text".to_string());
            }
        }

        DetectCandidate {
            codec: "urlencoding".to_string(),
            confidence,
            reasons,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode() {
        assert_eq!(UrlEncoding.encode(b"Hello").unwrap(), "Hello");
        assert_eq!(UrlEncoding.encode(b"Hello World").unwrap(), "Hello%20World");
        assert_eq!(UrlEncoding.encode(b"test@example.com").unwrap(), "test%40example.com");
        assert_eq!(UrlEncoding.encode(b"a+b=c").unwrap(), "a%2Bb%3Dc");
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(UrlEncoding.decode("Hello", Mode::Strict).unwrap(), b"Hello");
        assert_eq!(UrlEncoding.decode("Hello%20World", Mode::Strict).unwrap(), b"Hello World");
        assert_eq!(UrlEncoding.decode("test%40example.com", Mode::Strict).unwrap(), b"test@example.com");
    }

    #[test]
    fn test_url_roundtrip() {
        let data = b"Hello, World! @#$%^&*()";
        let encoded = UrlEncoding.encode(data).unwrap();
        assert_eq!(UrlEncoding.decode(&encoded, Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_url_unreserved() {
        let unreserved = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~";
        let encoded = UrlEncoding.encode(unreserved).unwrap();
        assert_eq!(encoded, std::str::from_utf8(unreserved).unwrap());
    }

    #[test]
    fn test_url_special_chars() {
        assert_eq!(UrlEncoding.encode(b" ").unwrap(), "%20");
        assert_eq!(UrlEncoding.encode(b"!").unwrap(), "%21");
        assert_eq!(UrlEncoding.encode(b"#").unwrap(), "%23");
        assert_eq!(UrlEncoding.encode(b"$").unwrap(), "%24");
    }

    #[test]
    fn test_url_invalid_sequence() {
        assert!(UrlEncoding.decode("%2", Mode::Strict).is_err());
        assert!(UrlEncoding.decode("%", Mode::Strict).is_err());
        assert!(UrlEncoding.decode("%ZZ", Mode::Strict).is_err());
    }

    #[test]
    fn test_url_empty() {
        assert_eq!(UrlEncoding.encode(&[]).unwrap(), "");
        assert_eq!(UrlEncoding.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_url_lowercase_hex() {
        assert_eq!(UrlEncoding.decode("test%20example", Mode::Lenient).unwrap(), b"test example");
        assert_eq!(UrlEncoding.decode("test%2fpath", Mode::Lenient).unwrap(), b"test/path");
    }

    #[test]
    fn test_url_utf8() {
        let utf8_bytes = "Hello 世界".as_bytes();
        let encoded = UrlEncoding.encode(utf8_bytes).unwrap();
        assert_eq!(UrlEncoding.decode(&encoded, Mode::Strict).unwrap(), utf8_bytes);
    }
}
