use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const LOWER_ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyz";
const UPPER_ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn encode_base36(input: &[u8], alphabet: &[u8]) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut num = input.iter().fold(Vec::new(), |mut acc, &byte| {
        let mut carry = byte as u32;
        for digit in acc.iter_mut() {
            carry += (*digit as u32) << 8;
            *digit = (carry % 36) as u8;
            carry /= 36;
        }
        while carry > 0 {
            acc.push((carry % 36) as u8);
            carry /= 36;
        }
        acc
    });

    let leading_zeros = input.iter().take_while(|&&b| b == 0).count();
    num.extend(std::iter::repeat_n(0, leading_zeros));

    num.iter().rev().map(|&d| alphabet[d as usize] as char).collect()
}

fn decode_base36(input: &str, mode: Mode, is_lowercase: bool) -> Result<Vec<u8>> {
    let cleaned = util::clean_for_mode(input, mode);

    if cleaned.is_empty() {
        return Ok(Vec::new());
    }

    let normalized = match mode {
        Mode::Strict => cleaned,
        Mode::Lenient => {
            if is_lowercase {
                cleaned.to_lowercase()
            } else {
                cleaned.to_uppercase()
            }
        }
    };

    let alphabet = if is_lowercase { LOWER_ALPHABET } else { UPPER_ALPHABET };

    for (pos, ch) in normalized.chars().enumerate() {
        if !alphabet.contains(ch) {
            return Err(MbaseError::InvalidCharacter { char: ch, position: pos });
        }
    }

    let leading_zeros = normalized.chars().take_while(|&c| c == '0').count();

    let mut result = normalized.chars().fold(Vec::new(), |mut acc, ch| {
        let digit = if ch.is_ascii_digit() {
            ch as u8 - b'0'
        } else if ch.is_ascii_lowercase() {
            ch as u8 - b'a' + 10
        } else {
            ch as u8 - b'A' + 10
        };

        let mut carry = digit as u32;
        for byte in acc.iter_mut().rev() {
            carry += (*byte as u32) * 36;
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

fn validate_base36(input: &str, alphabet: &str, mode: Mode) -> Result<()> {
    let cleaned = util::clean_for_mode(input, mode);

    for (pos, ch) in cleaned.chars().enumerate() {
        let valid = match mode {
            Mode::Strict => alphabet.contains(ch),
            Mode::Lenient => LOWER_ALPHABET.contains(ch.to_ascii_lowercase()),
        };
        if !valid {
            return Err(MbaseError::InvalidCharacter { char: ch, position: pos });
        }
    }
    Ok(())
}

fn detect_base36(input: &str, codec_name: &str, multibase_code: char) -> DetectCandidate {
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();
    let warnings = Vec::new();

    if input.is_empty() {
        return DetectCandidate {
            codec: codec_name.to_string(),
            confidence: 0.0,
            reasons: vec!["empty input".to_string()],
            warnings: vec![],
        };
    }

    if input.starts_with(multibase_code) {
        confidence = util::confidence::MULTIBASE_MATCH;
        reasons.push(format!("multibase prefix '{}' detected", multibase_code));
    }

    let valid = input.chars().filter(|c| c.is_ascii_alphanumeric()).count();
    let ratio = valid as f64 / input.len() as f64;

    if ratio == 1.0 {
        confidence = confidence.max(util::confidence::PARTIAL_MATCH);
        reasons.push("all characters alphanumeric".to_string());
    }

    DetectCandidate {
        codec: codec_name.to_string(),
        confidence: confidence.min(1.0),
        reasons,
        warnings,
    }
}

pub struct Base36Lower;

impl Codec for Base36Lower {
    fn name(&self) -> &'static str {
        "base36lower"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base36lower",
            aliases: &["base36", "b36"],
            alphabet: LOWER_ALPHABET,
            multibase_code: Some('k'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Lower,
            description: "Base36 lowercase (0-9a-z)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(encode_base36(input, LOWER_ALPHABET.as_bytes()))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_base36(input, mode, true)
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        validate_base36(input, LOWER_ALPHABET, mode)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_base36(input, "base36lower", 'k')
    }
}

pub struct Base36Upper;

impl Codec for Base36Upper {
    fn name(&self) -> &'static str {
        "base36upper"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base36upper",
            aliases: &["B36"],
            alphabet: UPPER_ALPHABET,
            multibase_code: Some('K'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Upper,
            description: "Base36 uppercase (0-9A-Z)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(encode_base36(input, UPPER_ALPHABET.as_bytes()))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_base36(input, mode, false)
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        validate_base36(input, UPPER_ALPHABET, mode)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_base36(input, "base36upper", 'K')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base36_encode() {
        assert_eq!(Base36Lower.encode(b"Hello").unwrap(), "3yud78mn");
        assert_eq!(Base36Upper.encode(b"Hello").unwrap(), "3YUD78MN");
    }

    #[test]
    fn test_base36_decode() {
        assert_eq!(Base36Lower.decode("3yud78mn", Mode::Strict).unwrap(), b"Hello");
        assert_eq!(Base36Upper.decode("3YUD78MN", Mode::Strict).unwrap(), b"Hello");
    }

    #[test]
    fn test_base36_roundtrip() {
        let data = b"The quick brown fox";
        let encoded = Base36Lower.encode(data).unwrap();
        let decoded = Base36Lower.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base36_leading_zeros() {
        let data = b"\x00\x00Hello";
        let encoded = Base36Lower.encode(data).unwrap();
        let decoded = Base36Lower.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base36_lenient_case() {
        assert_eq!(Base36Lower.decode("3YUD78MN", Mode::Lenient).unwrap(), b"Hello");
    }

    #[test]
    fn test_base36_empty() {
        assert_eq!(Base36Lower.encode(&[]).unwrap(), "");
        assert_eq!(Base36Lower.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_base36_strict_rejects_wrong_case() {
        assert!(Base36Lower.validate("3YUD78MN", Mode::Strict).is_err());
    }

    #[test]
    fn test_base36_leading_zeros_multiple() {
        let data = b"\x00\x00\x00\x01";
        let encoded = Base36Lower.encode(data).unwrap();
        let decoded = Base36Lower.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data, "Failed roundtrip for [0,0,0,1]");
    }

    #[test]
    fn test_base36_leading_zeros_all_zeros() {
        let data = b"\x00\x00\x00";
        let encoded = Base36Lower.encode(data).unwrap();
        assert_eq!(encoded, "000", "Should encode as three zeros");
        let decoded = Base36Lower.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data, "Failed roundtrip for [0,0,0]");
    }

    #[test]
    fn test_base36_no_leading_zero_byte_but_leading_zero_digit() {
        // [1] encodes to "1", but what if we have a value that starts with '0' digit?
        // Actually base36 encodes [0,1] differently than [1]
        let data1 = b"\x00\x01";
        let data2 = b"\x01";
        let enc1 = Base36Lower.encode(data1).unwrap();
        let enc2 = Base36Lower.encode(data2).unwrap();
        assert_ne!(enc1, enc2, "Leading zero byte should produce different encoding");

        let dec1 = Base36Lower.decode(&enc1, Mode::Strict).unwrap();
        let dec2 = Base36Lower.decode(&enc2, Mode::Strict).unwrap();
        assert_eq!(dec1, data1);
        assert_eq!(dec2, data2);
    }

    #[test]
    fn test_base36_roundtrip_various_patterns() {
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
            let encoded = Base36Lower.encode(&data).unwrap();
            let decoded = Base36Lower.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, data, "Failed roundtrip for {:?}", data);
        }
    }
}
