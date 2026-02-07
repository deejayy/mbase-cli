use data_encoding::{Encoding, HEXLOWER, HEXLOWER_PERMISSIVE, HEXUPPER, HEXUPPER_PERMISSIVE};

use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const LOWER_ALPHABET: &str = "0123456789abcdef";
const UPPER_ALPHABET: &str = "0123456789ABCDEF";

fn decode_hex(input: &str, mode: Mode, strict_enc: &Encoding, lenient_enc: &Encoding) -> Result<Vec<u8>> {
    let cleaned = util::clean_for_mode(input, mode);

    let to_decode = if mode == Mode::Lenient && cleaned.starts_with("0x") {
        &cleaned[2..]
    } else {
        &cleaned
    };

    if to_decode.len() % 2 != 0 {
        return Err(MbaseError::invalid_length(crate::error::LengthConstraint::MultipleOf(2), to_decode.len()));
    }

    let enc = match mode {
        Mode::Strict => strict_enc,
        Mode::Lenient => lenient_enc,
    };

    enc.decode(to_decode.as_bytes())
        .map_err(|e| MbaseError::invalid_input(e.to_string()))
}

fn detect_hex(input: &str, codec_name: &str, multibase_code: char) -> DetectCandidate {
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();
    let mut warnings = Vec::new();

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

    let hex_chars: usize = input.chars().filter(|c| c.is_ascii_hexdigit()).count();
    let ratio = hex_chars as f64 / input.len() as f64;

    if ratio == 1.0 {
        confidence = confidence.max(util::confidence::ALPHABET_MATCH);
        reasons.push("all characters are hex digits".to_string());
    } else if ratio >= 0.9 {
        confidence = confidence.max(util::confidence::WEAK_MATCH);
        warnings.push(format!("{:.1}% non-hex characters", (1.0 - ratio) * 100.0));
    }

    if !input.len().is_multiple_of(2) {
        confidence *= 0.5;
        warnings.push("odd length".to_string());
    }

    DetectCandidate {
        codec: codec_name.to_string(),
        confidence: confidence.min(1.0),
        reasons,
        warnings,
    }
}

pub struct Base16Lower;

impl Codec for Base16Lower {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base16lower",
            aliases: &["hex", "base16", "hexlower"],
            alphabet: LOWER_ALPHABET,
            multibase_code: Some('f'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Lower,
            description: "RFC4648 Base16 lowercase (hex)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(HEXLOWER.encode(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_hex(input, mode, &HEXLOWER, &HEXLOWER_PERMISSIVE)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_hex(input, "base16lower", 'f')
    }
}

pub struct Base16Upper;

impl Codec for Base16Upper {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base16upper",
            aliases: &["hexupper", "HEX"],
            alphabet: UPPER_ALPHABET,
            multibase_code: Some('F'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Upper,
            description: "RFC4648 Base16 uppercase",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(HEXUPPER.encode(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_hex(input, mode, &HEXUPPER, &HEXUPPER_PERMISSIVE)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_hex(input, "base16upper", 'F')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base16_encode() {
        assert_eq!(Base16Lower.encode(b"Hello").unwrap(), "48656c6c6f");
        assert_eq!(Base16Upper.encode(b"Hello").unwrap(), "48656C6C6F");
    }

    #[test]
    fn test_base16_decode() {
        assert_eq!(Base16Lower.decode("48656c6c6f", Mode::Strict).unwrap(), b"Hello");
        assert_eq!(Base16Upper.decode("48656C6C6F", Mode::Strict).unwrap(), b"Hello");
    }

    #[test]
    fn test_base16_roundtrip() {
        let data = b"\x00\xff\x7f\x80";
        assert_eq!(Base16Lower.decode(&Base16Lower.encode(data).unwrap(), Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_base16_lenient_case() {
        assert_eq!(Base16Lower.decode("48656C6C6F", Mode::Lenient).unwrap(), b"Hello");
    }

    #[test]
    fn test_base16_lenient_prefix() {
        assert_eq!(Base16Lower.decode("0x48656c6c6f", Mode::Lenient).unwrap(), b"Hello");
    }

    #[test]
    fn test_base16_lenient_whitespace() {
        assert_eq!(Base16Lower.decode("4865 6c6c 6f", Mode::Lenient).unwrap(), b"Hello");
    }

    #[test]
    fn test_base16_strict_rejects_wrong_case() {
        assert!(Base16Lower.validate("48656C6C6F", Mode::Strict).is_err());
    }

    #[test]
    fn test_base16_odd_length() {
        assert!(Base16Lower.decode("4865a", Mode::Strict).is_err());
    }

    #[test]
    fn test_base16_empty() {
        assert_eq!(Base16Lower.encode(&[]).unwrap(), "");
        assert_eq!(Base16Lower.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }
}
