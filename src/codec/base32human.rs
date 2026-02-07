use data_encoding::{Encoding, Specification};
use std::sync::OnceLock;

use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const ZBASE32_ALPHABET_FULL: &str = "ybndrfg8ejkmcpqxot1uwisza345h769";
const CROCKFORD_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

static ZBASE32_ENCODING: OnceLock<Encoding> = OnceLock::new();

fn get_zbase32() -> &'static Encoding {
    ZBASE32_ENCODING.get_or_init(|| {
        let mut spec = Specification::new();
        spec.symbols.push_str(ZBASE32_ALPHABET_FULL);
        spec.encoding().unwrap()
    })
}

fn crockford_encode(input: &[u8]) -> String {
    if input.is_empty() {
        return String::new();
    }

    let alphabet = CROCKFORD_ALPHABET.as_bytes();
    let mut result = String::new();
    let mut buffer: u64 = 0;
    let mut bits = 0;

    for &byte in input {
        buffer = (buffer << 8) | (byte as u64);
        bits += 8;

        while bits >= 5 {
            bits -= 5;
            let idx = ((buffer >> bits) & 0x1f) as usize;
            result.push(alphabet[idx] as char);
        }
    }

    if bits > 0 {
        let idx = ((buffer << (5 - bits)) & 0x1f) as usize;
        result.push(alphabet[idx] as char);
    }

    result
}

fn crockford_decode(input: &str, mode: Mode) -> Result<Vec<u8>> {
    let cleaned: String = match mode {
        Mode::Strict => input.to_string(),
        Mode::Lenient => input
            .chars()
            .filter(|c| !c.is_ascii_whitespace() && *c != '-')
            .collect(),
    };

    if cleaned.is_empty() {
        return Ok(Vec::new());
    }

    let mut buffer: u64 = 0;
    let mut bits = 0;
    let mut result = Vec::new();

    for (pos, ch) in cleaned.chars().enumerate() {
        let val = crockford_char_value(ch, mode)?;
        if val.is_none() {
            return Err(MbaseError::InvalidCharacter { char: ch, position: pos });
        }

        buffer = (buffer << 5) | (val.unwrap() as u64);
        bits += 5;

        while bits >= 8 {
            bits -= 8;
            result.push(((buffer >> bits) & 0xff) as u8);
        }
    }

    // Validate that any remaining bits are zero (padding)
    if bits > 0 {
        let remaining_bits = buffer & ((1 << bits) - 1);
        if remaining_bits != 0 {
            return Err(MbaseError::invalid_input(
                "crockford32 decode: non-zero padding bits"
            ));
        }
    }

    Ok(result)
}

fn crockford_char_value(ch: char, mode: Mode) -> Result<Option<u8>> {
    let val = match ch.to_ascii_uppercase() {
        '0' | 'O' if mode == Mode::Lenient => Some(0),
        '0' => Some(0),
        '1' | 'I' | 'L' if mode == Mode::Lenient => Some(1),
        '1' => Some(1),
        '2' => Some(2),
        '3' => Some(3),
        '4' => Some(4),
        '5' => Some(5),
        '6' => Some(6),
        '7' => Some(7),
        '8' => Some(8),
        '9' => Some(9),
        'A' => Some(10),
        'B' => Some(11),
        'C' => Some(12),
        'D' => Some(13),
        'E' => Some(14),
        'F' => Some(15),
        'G' => Some(16),
        'H' => Some(17),
        'J' => Some(18),
        'K' => Some(19),
        'M' => Some(20),
        'N' => Some(21),
        'P' => Some(22),
        'Q' => Some(23),
        'R' => Some(24),
        'S' => Some(25),
        'T' => Some(26),
        'V' => Some(27),
        'W' => Some(28),
        'X' => Some(29),
        'Y' => Some(30),
        'Z' => Some(31),
        'O' | 'I' | 'L' if mode == Mode::Strict => None,
        _ => None,
    };
    Ok(val)
}

fn validate_crockford(input: &str, mode: Mode) -> Result<()> {
    let cleaned: String = match mode {
        Mode::Strict => input.to_string(),
        Mode::Lenient => input
            .chars()
            .filter(|c| !c.is_ascii_whitespace() && *c != '-')
            .collect(),
    };

    for (pos, ch) in cleaned.chars().enumerate() {
        let upper = ch.to_ascii_uppercase();
        let valid = match mode {
            Mode::Strict => CROCKFORD_ALPHABET.contains(upper) && ch.is_ascii_uppercase(),
            Mode::Lenient => {
                CROCKFORD_ALPHABET.contains(upper)
                    || upper == 'O'
                    || upper == 'I'
                    || upper == 'L'
            }
        };
        if !valid {
            return Err(MbaseError::InvalidCharacter { char: ch, position: pos });
        }
    }
    Ok(())
}

pub struct ZBase32;

impl Codec for ZBase32 {
    fn name(&self) -> &'static str {
        "zbase32"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "zbase32",
            aliases: &["z32", "base32z"],
            alphabet: ZBASE32_ALPHABET_FULL,
            multibase_code: Some('h'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Lower,
            description: "z-base-32 human-oriented encoding",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(get_zbase32().encode(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned: String = match mode {
            Mode::Strict => input.to_string(),
            Mode::Lenient => input.chars().filter(|c| !c.is_ascii_whitespace()).collect(),
        };
        get_zbase32()
            .decode(cleaned.as_bytes())
            .map_err(|e| MbaseError::invalid_input(e.to_string()))
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        let cleaned: String = match mode {
            Mode::Strict => input.to_string(),
            Mode::Lenient => input.chars().filter(|c| !c.is_ascii_whitespace()).collect(),
        };
        for (pos, ch) in cleaned.chars().enumerate() {
            if !ZBASE32_ALPHABET_FULL.contains(ch) {
                return Err(MbaseError::InvalidCharacter { char: ch, position: pos });
            }
        }
        Ok(())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence: f64 = 0.0;
        let mut reasons = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "zbase32".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        if input.starts_with('h') {
            confidence = util::confidence::MULTIBASE_MATCH;
            reasons.push("multibase prefix 'h' detected".to_string());
        }

        let valid = input
            .chars()
            .filter(|c| ZBASE32_ALPHABET_FULL.contains(*c))
            .count();
        let ratio = valid as f64 / input.len() as f64;

        if ratio == 1.0 {
            confidence = confidence.max(util::confidence::PARTIAL_MATCH);
            reasons.push("all characters valid".to_string());
        }

        DetectCandidate {
            codec: "zbase32".to_string(),
            confidence: confidence.min(1.0),
            reasons,
            warnings: vec![],
        }
    }
}

pub struct Crockford32;

impl Codec for Crockford32 {
    fn name(&self) -> &'static str {
        "crockford32"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "crockford32",
            aliases: &["crockford", "cf32"],
            alphabet: CROCKFORD_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Upper,
            description: "Crockford's Base32 (human-friendly, no I/L/O/U)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(crockford_encode(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        crockford_decode(input, mode)
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        validate_crockford(input, mode)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence: f64 = 0.0;
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "crockford32".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let valid = input
            .chars()
            .filter(|c| CROCKFORD_ALPHABET.contains(c.to_ascii_uppercase()))
            .count();
        let ratio = valid as f64 / input.len() as f64;

        if ratio == 1.0 {
            confidence = util::confidence::PARTIAL_MATCH;
            reasons.push("all characters valid".to_string());
        }

        if input.chars().any(|c| c == 'I' || c == 'L' || c == 'O') {
            warnings.push("contains confusable characters (I/L/O)".to_string());
        }

        DetectCandidate {
            codec: "crockford32".to_string(),
            confidence: confidence.min(1.0),
            reasons,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zbase32_encode() {
        assert_eq!(ZBase32.encode(b"Hello").unwrap(), "jb1sa5dx");
    }

    #[test]
    fn test_zbase32_decode() {
        assert_eq!(ZBase32.decode("jb1sa5dx", Mode::Strict).unwrap(), b"Hello");
    }

    #[test]
    fn test_zbase32_roundtrip() {
        let data = b"The quick brown fox";
        let encoded = ZBase32.encode(data).unwrap();
        let decoded = ZBase32.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_crockford_encode() {
        assert_eq!(Crockford32.encode(b"Hello").unwrap(), "91JPRV3F");
    }

    #[test]
    fn test_crockford_decode() {
        assert_eq!(
            Crockford32.decode("91JPRV3F", Mode::Strict).unwrap(),
            b"Hello"
        );
    }

    #[test]
    fn test_crockford_roundtrip() {
        let data = b"The quick brown fox";
        let encoded = Crockford32.encode(data).unwrap();
        let decoded = Crockford32.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_crockford_lenient_confusables() {
        assert_eq!(
            Crockford32.decode("O1JPRV3F", Mode::Lenient).unwrap(),
            Crockford32.decode("01JPRV3F", Mode::Lenient).unwrap()
        );
        assert_eq!(
            Crockford32.decode("9IJPRV3F", Mode::Lenient).unwrap(),
            Crockford32.decode("91JPRV3F", Mode::Lenient).unwrap()
        );
    }

    #[test]
    fn test_crockford_lenient_hyphens() {
        assert_eq!(
            Crockford32.decode("91JP-RV3F", Mode::Lenient).unwrap(),
            b"Hello"
        );
    }

    #[test]
    fn test_crockford_lenient_lowercase() {
        assert_eq!(
            Crockford32.decode("91jprv3f", Mode::Lenient).unwrap(),
            b"Hello"
        );
    }

    #[test]
    fn test_crockford_strict_rejects_lowercase() {
        assert!(Crockford32.validate("91jprv3f", Mode::Strict).is_err());
    }

    #[test]
    fn test_crockford_empty() {
        assert_eq!(Crockford32.encode(&[]).unwrap(), "");
        assert_eq!(
            Crockford32.decode("", Mode::Strict).unwrap(),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn test_crockford_roundtrip_various_lengths() {
        // Test various byte lengths to ensure trailing bits are handled correctly
        for len in 0..=20 {
            let data: Vec<u8> = (0..len).map(|i| (i * 17) as u8).collect();
            let encoded = Crockford32.encode(&data).unwrap();
            let decoded = Crockford32.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, data, "roundtrip failed for length {}", len);
        }
    }

    #[test]
    fn test_crockford_reject_invalid_padding() {
        // Create a string with an extra character that would create invalid trailing bits
        // "91JPRV3F" = "Hello" (5 bytes = 40 bits)
        // 40 bits / 5 bits per char = 8 chars exactly, no padding
        // Let's try with 3 bytes = 24 bits, encoded to 5 chars (25 bits), leaving 1 bit padding
        let three_bytes = b"Hel";
        let encoded = Crockford32.encode(three_bytes).unwrap();
        // Now modify last char to create non-zero padding
        let mut chars: Vec<char> = encoded.chars().collect();
        let last_idx = chars.len() - 1;
        // Change last char to one that would set trailing bits differently
        let last_val = crockford_char_value(chars[last_idx], Mode::Strict).unwrap().unwrap();
        let new_val = (last_val ^ 1) & 0x1f; // Flip bit 0
        // Find corresponding char for new_val
        let new_char = CROCKFORD_ALPHABET.chars().nth(new_val as usize).unwrap();
        chars[last_idx] = new_char;
        let modified: String = chars.into_iter().collect();
        
        let result = Crockford32.decode(&modified, Mode::Strict);
        assert!(result.is_err(), "should reject invalid padding bits");
    }
}
