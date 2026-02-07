use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn encode_base62(input: &[u8]) -> String {
    if input.is_empty() {
        return String::new();
    }

    let alphabet = ALPHABET.as_bytes();
    let mut num = input.iter().fold(Vec::new(), |mut acc, &byte| {
        let mut carry = byte as u32;
        for digit in acc.iter_mut() {
            carry += (*digit as u32) << 8;
            *digit = (carry % 62) as u8;
            carry /= 62;
        }
        while carry > 0 {
            acc.push((carry % 62) as u8);
            carry /= 62;
        }
        acc
    });

    let leading_zeros = input.iter().take_while(|&&b| b == 0).count();
    for _ in 0..leading_zeros {
        num.push(0);
    }

    num.iter()
        .rev()
        .map(|&d| alphabet[d as usize] as char)
        .collect()
}

fn decode_base62(input: &str, mode: Mode) -> Result<Vec<u8>> {
    let cleaned = util::clean_for_mode(input, mode);

    if cleaned.is_empty() {
        return Ok(Vec::new());
    }

    for (pos, ch) in cleaned.chars().enumerate() {
        if !ALPHABET.contains(ch) {
            return Err(MbaseError::InvalidCharacter { char: ch, position: pos });
        }
    }

    let leading_zeros = cleaned.chars().take_while(|&c| c == '0').count();

    let mut result = cleaned.chars().fold(Vec::new(), |mut acc, ch| {
        let digit = if ch.is_ascii_digit() {
            ch as u8 - b'0'
        } else if ch.is_ascii_uppercase() {
            ch as u8 - b'A' + 10
        } else {
            ch as u8 - b'a' + 36
        };

        let mut carry = digit as u32;
        for byte in acc.iter_mut().rev() {
            carry += (*byte as u32) * 62;
            *byte = (carry & 0xff) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            acc.insert(0, (carry & 0xff) as u8);
            carry >>= 8;
        }
        acc
    });

    let mut output = vec![0u8; leading_zeros];
    output.append(&mut result);
    Ok(output)
}

fn validate_base62(input: &str, mode: Mode) -> Result<()> {
    let cleaned = util::clean_for_mode(input, mode);

    for (pos, ch) in cleaned.chars().enumerate() {
        if !ALPHABET.contains(ch) {
            return Err(MbaseError::InvalidCharacter { char: ch, position: pos });
        }
    }
    Ok(())
}

fn detect_base62(input: &str) -> DetectCandidate {
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();

    if input.is_empty() {
        return DetectCandidate {
            codec: "base62".to_string(),
            confidence: 0.0,
            reasons: vec!["empty input".to_string()],
            warnings: vec![],
        };
    }

    let valid = input.chars().filter(|c| ALPHABET.contains(*c)).count();
    let ratio = valid as f64 / input.len() as f64;

    if ratio == 1.0 {
        confidence = util::confidence::WEAK_MATCH;
        reasons.push("all characters alphanumeric".to_string());
    }

    DetectCandidate {
        codec: "base62".to_string(),
        confidence: confidence.min(1.0),
        reasons,
        warnings: vec!["base62 has no standard; low confidence".to_string()],
    }
}

pub struct Base62;

impl Codec for Base62 {
    fn name(&self) -> &'static str {
        "base62"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base62",
            aliases: &["b62"],
            alphabet: ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Base62 (0-9A-Za-z) big-integer encoding",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(encode_base62(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_base62(input, mode)
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        validate_base62(input, mode)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_base62(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base62_encode() {
        let encoded = Base62.encode(b"Hello").unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded.chars().all(|c| ALPHABET.contains(c)));
    }

    #[test]
    fn test_base62_roundtrip() {
        let data = b"The quick brown fox";
        let encoded = Base62.encode(data).unwrap();
        let decoded = Base62.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base62_empty() {
        assert_eq!(Base62.encode(&[]).unwrap(), "");
        assert_eq!(Base62.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_base62_leading_zeros() {
        let data = b"\x00\x00Hello";
        let encoded = Base62.encode(data).unwrap();
        assert!(encoded.starts_with("00"));
        let decoded = Base62.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base62_single_byte() {
        for byte in [0u8, 1, 127, 255] {
            let encoded = Base62.encode(&[byte]).unwrap();
            let decoded = Base62.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, &[byte]);
        }
    }

    #[test]
    fn test_base62_invalid_char() {
        assert!(Base62.decode("Hello+World", Mode::Strict).is_err());
    }

    #[test]
    fn test_base62_roundtrip_various_patterns() {
        let test_cases = vec![
            vec![0],
            vec![0, 0],
            vec![0, 0, 0],
            vec![0, 1],
            vec![0, 0, 1],
            vec![0, 0, 0, 1],
            vec![1, 0],
            vec![0, 255],
            vec![255, 0],
            vec![0, 0, 255],
        ];

        for data in test_cases {
            let encoded = Base62.encode(&data).unwrap();
            let decoded = Base62.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, data, "Failed roundtrip for {:?}", data);
        }
    }

    #[test]
    fn test_base62_lenient_whitespace() {
        let encoded = Base62.encode(b"Test").unwrap();
        let with_space = format!("{} ", encoded);
        let decoded = Base62.decode(&with_space, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"Test");
    }

    #[test]
    fn test_base62_validate() {
        assert!(Base62.validate("ABCabc123", Mode::Strict).is_ok());
        assert!(Base62.validate("ABC+abc", Mode::Strict).is_err());
    }

    #[test]
    fn test_base62_detect_low_confidence() {
        let candidate = detect_base62("HelloWorld123");
        assert!(candidate.confidence <= 0.4);
        assert!(!candidate.warnings.is_empty());
    }
}
