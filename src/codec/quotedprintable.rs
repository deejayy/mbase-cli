use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

fn hex_value(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        'A'..='F' => Some(c as u8 - b'A' + 10),
        'a'..='f' => Some(c as u8 - b'a' + 10),
        _ => None,
    }
}

fn is_safe_char(b: u8) -> bool {
    matches!(b, 33..=60 | 62..=126) && b != b'='
}

pub struct QuotedPrintable;

impl Codec for QuotedPrintable {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "quoted-printable",
            aliases: &["qp"],
            alphabet: "printable ASCII + =XX hex escapes",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Quoted-Printable (RFC 2045) for email/MIME",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        if input.is_empty() {
            return Ok(String::new());
        }

        let mut result = String::new();
        let mut line_len = 0;

        for &byte in input {
            let encoded = if is_safe_char(byte) {
                let c = byte as char;
                if line_len >= 75 {
                    result.push_str("=\r\n");
                    line_len = 0;
                }
                result.push(c);
                line_len += 1;
                continue;
            } else {
                format!("={}{}", HEX_UPPER[(byte >> 4) as usize] as char, HEX_UPPER[(byte & 0x0F) as usize] as char)
            };

            if line_len + encoded.len() > 75 {
                result.push_str("=\r\n");
                line_len = 0;
            }
            result.push_str(&encoded);
            line_len += encoded.len();
        }

        Ok(result)
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let mut chars = input.chars().peekable();
        let mut pos = 0;

        while let Some(c) = chars.next() {
            if c == '=' {
                match (chars.next(), chars.peek()) {
                    (Some('\r'), Some(&'\n')) => {
                        chars.next();
                        pos += 3;
                    }
                    (Some('\n'), _) => {
                        pos += 2;
                    }
                    (Some(h1), Some(&h2)) => {
                        let v1 = hex_value(h1).ok_or_else(|| MbaseError::InvalidCharacter {
                            char: h1,
                            position: pos + 1,
                        })?;
                        let v2 = hex_value(h2).ok_or_else(|| MbaseError::InvalidCharacter {
                            char: h2,
                            position: pos + 2,
                        })?;
                        chars.next();
                        result.push((v1 << 4) | v2);
                        pos += 3;
                    }
                    (Some(c), None) if mode == Mode::Lenient => {
                        if let Some(v) = hex_value(c) {
                            result.push(v);
                        }
                        pos += 2;
                    }
                    _ => {
                        return Err(MbaseError::invalid_input(
                            "incomplete escape sequence",
                        ));
                    }
                }
            } else if c == '\r' || c == '\n' {
                if mode == Mode::Strict {
                    result.push(c as u8);
                }
                pos += 1;
            } else {
                result.push(c as u8);
                pos += 1;
            }
        }

        Ok(result)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        if input.is_empty() {
            return DetectCandidate {
                codec: self.name().to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let escape_count = input.matches("=").count();
        let mut valid_escapes = 0;
        let mut soft_breaks = 0;

        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '=' {
                match (chars.next(), chars.peek()) {
                    (Some('\r'), Some(&'\n')) => {
                        chars.next();
                        soft_breaks += 1;
                    }
                    (Some('\n'), _) => {
                        soft_breaks += 1;
                    }
                    (Some(h1), Some(&h2)) if hex_value(h1).is_some() && hex_value(h2).is_some() => {
                        chars.next();
                        valid_escapes += 1;
                    }
                    _ => {}
                }
            }
        }

        if escape_count == 0 {
            return DetectCandidate {
                codec: self.name().to_string(),
                confidence: 0.2,
                reasons: vec!["no escape sequences found".to_string()],
                warnings: vec![],
            };
        }

        let valid_ratio = (valid_escapes + soft_breaks) as f64 / escape_count as f64;
        let confidence = if valid_ratio > 0.8 {
            0.8
        } else if valid_ratio > 0.5 {
            util::confidence::ALPHABET_MATCH
        } else {
            util::confidence::WEAK_MATCH
        };

        DetectCandidate {
            codec: self.name().to_string(),
            confidence,
            reasons: vec![
                format!("{} valid escape sequences", valid_escapes),
                format!("{} soft line breaks", soft_breaks),
            ],
            warnings: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qp_empty() {
        let codec = QuotedPrintable;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_qp_ascii() {
        let codec = QuotedPrintable;
        let encoded = codec.encode(b"Hello World").unwrap();
        assert_eq!(encoded, "Hello=20World");
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_qp_special_chars() {
        let codec = QuotedPrintable;
        let encoded = codec.encode(b"caf\xc3\xa9").unwrap();
        assert!(encoded.contains("=C3"));
        assert!(encoded.contains("=A9"));
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"caf\xc3\xa9");
    }

    #[test]
    fn test_qp_equals_sign() {
        let codec = QuotedPrintable;
        let encoded = codec.encode(b"a=b").unwrap();
        assert_eq!(encoded, "a=3Db");
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"a=b");
    }

    #[test]
    fn test_qp_soft_line_break() {
        let codec = QuotedPrintable;
        let decoded = codec.decode("Hello=\r\nWorld", Mode::Strict).unwrap();
        assert_eq!(decoded, b"HelloWorld");
        let decoded = codec.decode("Hello=\nWorld", Mode::Strict).unwrap();
        assert_eq!(decoded, b"HelloWorld");
    }

    #[test]
    fn test_qp_roundtrip() {
        let codec = QuotedPrintable;
        let inputs = [
            b"".to_vec(),
            b"Hello".to_vec(),
            b"Hello World!".to_vec(),
            b"\x00\x01\x02".to_vec(),
            (0..=127).collect::<Vec<u8>>(),
        ];
        for input in inputs {
            let encoded = codec.encode(&input).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, input);
        }
    }

    #[test]
    fn test_qp_case_insensitive() {
        let codec = QuotedPrintable;
        let lower = codec.decode("=c3=a9", Mode::Strict).unwrap();
        let upper = codec.decode("=C3=A9", Mode::Strict).unwrap();
        assert_eq!(lower, upper);
    }

    #[test]
    fn test_qp_detect() {
        let codec = QuotedPrintable;
        let score = codec.detect_score("Hello=20World=3D=C3=A9");
        assert!(score.confidence >= 0.6);
    }
}
