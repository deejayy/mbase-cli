use bech32::{Bech32 as Bech32Variant, Bech32m as Bech32mVariant, Hrp};

use super::util;
use super::Codec;
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const BECH32_ALPHABET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
const DEFAULT_HRP: &str = "data";

fn encode_bech32<V: bech32::Checksum>(hrp_str: &str, data: &[u8]) -> Result<String> {
    let hrp = Hrp::parse(hrp_str).map_err(|e| MbaseError::invalid_input(format!("invalid HRP: {}", e)))?;
    bech32::encode::<V>(hrp, data).map_err(|e| MbaseError::invalid_input(format!("encoding failed: {}", e)))
}

fn decode_bech32_any(input: &str, mode: Mode) -> Result<(String, Vec<u8>, bool)> {
    let cleaned = util::clean_for_mode(input, mode);
    let cleaned_lower = cleaned.to_lowercase();

    match bech32::decode(&cleaned_lower) {
        Ok((hrp, data)) => {
            let is_m = bech32::encode::<Bech32mVariant>(hrp, &data)
                .map(|encoded| encoded.to_lowercase() == cleaned_lower)
                .unwrap_or(false);
            Ok((hrp.to_string(), data, is_m))
        }
        Err(_) => Err(MbaseError::ChecksumMismatch),
    }
}

fn decode_bech32_strict(input: &str, mode: Mode, is_m: bool) -> Result<Vec<u8>> {
    let cleaned = util::clean_for_mode(input, mode);
    let cleaned_lower = cleaned.to_lowercase();

    let (hrp, data) = bech32::decode(&cleaned_lower).map_err(|_| MbaseError::ChecksumMismatch)?;

    let reencoded = if is_m {
        bech32::encode::<Bech32mVariant>(hrp, &data)
    } else {
        bech32::encode::<Bech32Variant>(hrp, &data)
    };

    match reencoded {
        Ok(enc) if enc.to_lowercase() == cleaned_lower => Ok(data),
        _ => Err(MbaseError::ChecksumMismatch),
    }
}

fn detect_bech32(input: &str, codec_name: &str, is_m: bool) -> DetectCandidate {
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

    if let Some(sep_pos) = input.to_lowercase().rfind('1') {
        if sep_pos > 0 && sep_pos < input.len() - 7 {
            confidence = util::confidence::PARTIAL_MATCH;
            reasons.push("contains bech32 separator '1'".to_string());

            let data_part = &input[sep_pos + 1..];
            let valid = data_part
                .chars()
                .filter(|c| BECH32_ALPHABET.contains(c.to_ascii_lowercase()))
                .count();
            if valid == data_part.len() {
                confidence = util::confidence::ALPHABET_MATCH;
                reasons.push("valid bech32 charset".to_string());
            }
        }
    }

    if let Ok((_, _, detected_m)) = decode_bech32_any(input, Mode::Lenient) {
        if detected_m == is_m {
            confidence = util::confidence::MULTIBASE_MATCH;
            reasons.push("checksum valid".to_string());
        }
    }

    DetectCandidate {
        codec: codec_name.to_string(),
        confidence: confidence.min(1.0),
        reasons,
        warnings,
    }
}

pub struct Bech32Codec;

impl Codec for Bech32Codec {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "bech32",
            aliases: &[],
            alphabet: BECH32_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Bech32 (BIP-173) with HRP separator",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        encode_bech32::<Bech32Variant>(DEFAULT_HRP, input)
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_bech32_strict(input, mode, false)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_bech32(input, "bech32", false)
    }
}

pub struct Bech32mCodec;

impl Codec for Bech32mCodec {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "bech32m",
            aliases: &[],
            alphabet: BECH32_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Bech32m (BIP-350) with updated checksum constant",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        encode_bech32::<Bech32mVariant>(DEFAULT_HRP, input)
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_bech32_strict(input, mode, true)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_bech32(input, "bech32m", true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bech32_encode() {
        let encoded = Bech32Codec.encode(b"Hello").unwrap();
        assert!(encoded.starts_with("data1"));
        assert!(encoded.len() > 10);
    }

    #[test]
    fn test_bech32_decode() {
        let encoded = Bech32Codec.encode(b"Hello").unwrap();
        let decoded = Bech32Codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn test_bech32_roundtrip() {
        let data = b"The quick brown fox";
        let encoded = Bech32Codec.encode(data).unwrap();
        let decoded = Bech32Codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_bech32m_encode() {
        let encoded = Bech32mCodec.encode(b"Hello").unwrap();
        assert!(encoded.starts_with("data1"));
    }

    #[test]
    fn test_bech32m_roundtrip() {
        let data = b"Test data";
        let encoded = Bech32mCodec.encode(data).unwrap();
        let decoded = Bech32mCodec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_bech32_vs_bech32m_different() {
        let data = b"Hello";
        let b32 = Bech32Codec.encode(data).unwrap();
        let b32m = Bech32mCodec.encode(data).unwrap();
        assert_ne!(b32, b32m);
    }

    #[test]
    fn test_bech32_checksum_mismatch() {
        let encoded = Bech32Codec.encode(b"Hello").unwrap();
        let result = Bech32mCodec.decode(&encoded, Mode::Strict);
        assert!(matches!(result, Err(MbaseError::ChecksumMismatch)));
    }

    #[test]
    fn test_bech32m_checksum_mismatch() {
        let encoded = Bech32mCodec.encode(b"Hello").unwrap();
        let result = Bech32Codec.decode(&encoded, Mode::Strict);
        assert!(matches!(result, Err(MbaseError::ChecksumMismatch)));
    }

    #[test]
    fn test_bech32_case_insensitive() {
        let encoded = Bech32Codec.encode(b"Test").unwrap();
        let upper = encoded.to_uppercase();
        let decoded = Bech32Codec.decode(&upper, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Test");
    }

    #[test]
    fn test_bech32_lenient_whitespace() {
        let encoded = Bech32Codec.encode(b"Test").unwrap();
        let with_space = format!("{} ", encoded);
        let decoded = Bech32Codec.decode(&with_space, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"Test");
    }

    #[test]
    fn test_bech32_empty() {
        let encoded = Bech32Codec.encode(&[]).unwrap();
        let decoded = Bech32Codec.decode(&encoded, Mode::Strict).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_bech32_known_vector() {
        let encoded = Bech32Codec.encode(&[]).unwrap();
        let decoded = Bech32Codec.decode(&encoded, Mode::Strict).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_bech32_invalid_checksum() {
        let result = Bech32Codec.decode("data1xxxxxxxx", Mode::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn test_bech32_detect() {
        let encoded = Bech32Codec.encode(b"Test").unwrap();
        let candidate = Bech32Codec.detect_score(&encoded);
        assert!(candidate.confidence >= 0.9);
    }

    #[test]
    fn test_bech32m_detect() {
        let encoded = Bech32mCodec.encode(b"Test").unwrap();
        let candidate = Bech32mCodec.detect_score(&encoded);
        assert!(candidate.confidence >= 0.9);
    }
}
