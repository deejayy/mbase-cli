use super::{util, Codec};
use crate::error::{MbaseError as Error, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct Base37;

const ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ ";

fn encode_base37(input: &[u8]) -> String {
    if input.is_empty() {
        return String::new();
    }

    let alphabet = ALPHABET.as_bytes();
    let mut num = input.iter().fold(Vec::new(), |mut acc, &byte| {
        let mut carry = byte as u32;
        for digit in acc.iter_mut() {
            carry += (*digit as u32) << 8;
            *digit = (carry % 37) as u8;
            carry /= 37;
        }
        while carry > 0 {
            acc.push((carry % 37) as u8);
            carry /= 37;
        }
        acc
    });

    let leading_zeros = input.iter().take_while(|&&b| b == 0).count();
    num.extend(std::iter::repeat_n(0, leading_zeros));

    num.iter().rev().map(|&d| alphabet[d as usize] as char).collect()
}

fn decode_base37(input: &str, mode: Mode) -> Result<Vec<u8>> {
    let cleaned = if mode == Mode::Lenient {
        input.chars().filter(|c| !c.is_whitespace() || *c == ' ').collect::<String>()
    } else {
        input.to_string()
    };

    if cleaned.is_empty() {
        return Ok(Vec::new());
    }

    let normalized = cleaned.to_uppercase();

    for (pos, ch) in normalized.chars().enumerate() {
        if !ALPHABET.contains(ch) {
            return Err(Error::InvalidCharacter { char: ch, position: pos });
        }
    }

    let leading_zeros = normalized.chars().take_while(|&c| c == '0').count();

    let mut result = normalized.chars().fold(Vec::new(), |mut acc, ch| {
        let digit = ALPHABET.chars().position(|c| c == ch).unwrap() as u8;

        let mut carry = digit as u32;
        for byte in acc.iter_mut().rev() {
            carry += (*byte as u32) * 37;
            *byte = (carry & 0xff) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            acc.insert(0, (carry & 0xff) as u8);
            carry >>= 8;
        }
        acc
    });

    result.splice(0..0, std::iter::repeat_n(0, leading_zeros));
    Ok(result)
}

impl Codec for Base37 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base37",
            aliases: &[],
            alphabet: ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Base37 (Base36 + space character)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(encode_base37(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_base37(input, mode)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        if input.is_empty() {
            return DetectCandidate {
                codec: "base37".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            };
        }

        let valid_chars = input.chars().filter(|c| ALPHABET.contains(c.to_ascii_uppercase())).count();

        let ratio = valid_chars as f32 / input.len() as f32;

        if ratio > 0.95 && input.contains(' ') {
            DetectCandidate {
                codec: "base37".to_string(),
                confidence: util::confidence::PARTIAL_MATCH,
                reasons: vec!["high ratio with space".to_string()],
                warnings: vec![],
            }
        } else if ratio > 0.95 {
            DetectCandidate {
                codec: "base37".to_string(),
                confidence: util::confidence::WEAK_MATCH,
                reasons: vec!["high ratio".to_string()],
                warnings: vec![],
            }
        } else {
            DetectCandidate {
                codec: "base37".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base37_encode() {
        let codec = Base37;
        let encoded = codec.encode(b"hello").unwrap();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_base37_decode() {
        let codec = Base37;
        let encoded = codec.encode(b"hello").unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base37_roundtrip() {
        let codec = Base37;
        let test_cases = vec![
            b"test" as &[u8],
            b"hello world",
            b"The quick brown fox",
            &[0, 1, 2, 3, 4, 5],
            &[255, 254, 253],
        ];

        for original in test_cases {
            let encoded = codec.encode(original).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, original, "roundtrip failed for {:?}", original);
        }
    }

    #[test]
    fn test_base37_case_insensitive() {
        let codec = Base37;
        let upper = codec.decode("ABC", Mode::Strict).unwrap();
        let lower = codec.decode("abc", Mode::Strict).unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn test_base37_with_space() {
        let codec = Base37;
        let encoded = codec.encode(b"a b").unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"a b");
    }

    #[test]
    fn test_base37_empty() {
        let codec = Base37;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_base37_invalid_char() {
        let codec = Base37;
        assert!(codec.decode("ABC$DEF", Mode::Strict).is_err());
    }

    #[test]
    fn test_base37_detect() {
        let codec = Base37;
        assert!(codec.detect_score("ABC DEF").confidence > 0.4);
        assert!(codec.detect_score("ABC123").confidence > 0.2);
        assert!(codec.detect_score("hello$world").confidence < 0.1);
    }
}
