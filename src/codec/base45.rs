use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

fn char_to_val(c: char) -> Option<u32> {
    ALPHABET.chars().position(|x| x == c).map(|p| p as u32)
}

fn val_to_char(v: u32) -> char {
    ALPHABET.chars().nth(v as usize).unwrap()
}

fn encode_base45(input: &[u8]) -> String {
    let mut result = String::new();

    for chunk in input.chunks(2) {
        let n: u32 = if chunk.len() == 2 {
            (chunk[0] as u32) * 256 + (chunk[1] as u32)
        } else {
            chunk[0] as u32
        };

        let c = n % 45;
        let d = (n / 45) % 45;
        let e = n / (45 * 45);

        result.push(val_to_char(c));
        result.push(val_to_char(d));
        if chunk.len() == 2 {
            result.push(val_to_char(e));
        }
    }

    result
}

fn decode_base45(input: &str, mode: Mode) -> Result<Vec<u8>> {
    let cleaned = util::clean_for_mode(input, mode);

    if cleaned.is_empty() {
        return Ok(Vec::new());
    }

    let normalized = match mode {
        Mode::Strict => cleaned,
        Mode::Lenient => cleaned.to_uppercase(),
    };

    let vals: std::result::Result<Vec<u32>, _> = normalized
        .chars()
        .enumerate()
        .map(|(pos, c)| char_to_val(c).ok_or(MbaseError::InvalidCharacter { char: c, position: pos }))
        .collect();
    let vals = vals?;

    if vals.len() % 3 == 1 {
        return Err(MbaseError::invalid_input(format!("base45 length {} invalid (cannot be 1 mod 3)", vals.len())));
    }

    let mut result = Vec::new();

    for chunk in vals.chunks(3) {
        let n: u32 = if chunk.len() == 3 {
            chunk[0] + chunk[1] * 45 + chunk[2] * 45 * 45
        } else {
            chunk[0] + chunk[1] * 45
        };

        if chunk.len() == 3 {
            if n > 0xFFFF {
                return Err(MbaseError::invalid_input("base45 value overflow"));
            }
            result.push((n / 256) as u8);
            result.push((n % 256) as u8);
        } else {
            if n > 0xFF {
                return Err(MbaseError::invalid_input("base45 value overflow"));
            }
            result.push(n as u8);
        }
    }

    Ok(result)
}

fn validate_base45(input: &str, mode: Mode) -> Result<()> {
    let cleaned = util::clean_for_mode(input, mode);

    for (pos, c) in cleaned.chars().enumerate() {
        let valid = match mode {
            Mode::Strict => ALPHABET.contains(c),
            Mode::Lenient => ALPHABET.contains(c.to_ascii_uppercase()),
        };
        if !valid {
            return Err(MbaseError::InvalidCharacter { char: c, position: pos });
        }
    }

    if cleaned.len() % 3 == 1 {
        return Err(MbaseError::invalid_input(format!("base45 length {} invalid", cleaned.len())));
    }

    Ok(())
}

fn detect_base45(input: &str) -> DetectCandidate {
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();
    let warnings = Vec::new();

    if input.is_empty() {
        return DetectCandidate {
            codec: "base45".to_string(),
            confidence: 0.0,
            reasons: vec!["empty input".to_string()],
            warnings: vec![],
        };
    }

    let valid = input.chars().filter(|c| ALPHABET.contains(c.to_ascii_uppercase())).count();
    let ratio = valid as f64 / input.len() as f64;

    if ratio == 1.0 {
        confidence = util::confidence::PARTIAL_MATCH;
        reasons.push("all characters in base45 alphabet".to_string());

        if input.len() % 3 != 1 {
            confidence = util::confidence::PARTIAL_MATCH;
            reasons.push("valid length".to_string());
        }
    }

    if input.contains(' ') || input.contains('%') || input.contains('$') {
        confidence = confidence.max(util::confidence::ALPHABET_MATCH);
        reasons.push("contains base45-specific characters".to_string());
    }

    DetectCandidate {
        codec: "base45".to_string(),
        confidence: confidence.min(1.0),
        reasons,
        warnings,
    }
}

pub struct Base45;

impl Codec for Base45 {
    fn name(&self) -> &'static str {
        "base45"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base45",
            aliases: &["b45"],
            alphabet: ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Upper,
            description: "Base45 (RFC 9285) QR-code friendly encoding",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(encode_base45(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_base45(input, mode)
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        validate_base45(input, mode)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_base45(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base45_encode_hello() {
        let encoded = Base45.encode(b"Hello").unwrap();
        let decoded = Base45.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn test_base45_decode_hello() {
        let encoded = Base45.encode(b"Hello").unwrap();
        assert_eq!(Base45.decode(&encoded, Mode::Strict).unwrap(), b"Hello");
    }

    #[test]
    fn test_base45_roundtrip() {
        let data = b"The quick brown fox";
        let encoded = Base45.encode(data).unwrap();
        let decoded = Base45.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base45_empty() {
        assert_eq!(Base45.encode(&[]).unwrap(), "");
        assert_eq!(Base45.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_base45_single_byte() {
        let encoded = Base45.encode(&[0xAB]).unwrap();
        let decoded = Base45.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, &[0xAB]);
    }

    #[test]
    fn test_base45_rfc_vector_ab() {
        assert_eq!(Base45.encode(b"AB").unwrap(), "BB8");
    }

    #[test]
    fn test_base45_rfc_vector_ietf() {
        assert_eq!(Base45.encode(b"ietf!").unwrap(), "QED8WEX0");
    }

    #[test]
    fn test_base45_lenient_whitespace() {
        let encoded = Base45.encode(b"AB").unwrap();
        let with_space = format!("{} {}", &encoded[..2], &encoded[2..]);
        let decoded = Base45.decode(&with_space, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"AB");
    }

    #[test]
    fn test_base45_lenient_lowercase() {
        assert_eq!(Base45.decode("bb8", Mode::Lenient).unwrap(), b"AB");
    }

    #[test]
    fn test_base45_strict_rejects_lowercase() {
        assert!(Base45.decode("bb8", Mode::Strict).is_err());
    }

    #[test]
    fn test_base45_invalid_length() {
        assert!(Base45.decode("A", Mode::Strict).is_err());
    }

    #[test]
    fn test_base45_invalid_char() {
        assert!(Base45.decode("AB#", Mode::Strict).is_err());
    }

    #[test]
    fn test_base45_overflow() {
        let result = Base45.decode(":::", Mode::Strict);
        assert!(result.is_err());
    }
}
