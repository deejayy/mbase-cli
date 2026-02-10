use super::{util, Codec};
use crate::error::{MbaseError as Error, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct Base58Ripple;

const RIPPLE_ALPHABET: &str = "rpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2bcdeCg65jkm8oFqi1tuvAxyz";
const RIPPLE_ALPHABET_BYTES: &[u8; 58] = b"rpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2bcdeCg65jkm8oFqi1tuvAxyz";

impl Codec for Base58Ripple {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base58ripple",
            aliases: &["base58xrp"],
            alphabet: RIPPLE_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Base58 Ripple (XRP) alphabet",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let alphabet = bs58::Alphabet::new(RIPPLE_ALPHABET_BYTES).map_err(|e| Error::invalid_input(format!("invalid alphabet: {}", e)))?;
        Ok(bs58::encode(input).with_alphabet(&alphabet).into_string())
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);
        let alphabet = bs58::Alphabet::new(RIPPLE_ALPHABET_BYTES).map_err(|e| Error::invalid_input(format!("invalid alphabet: {}", e)))?;
        bs58::decode(&cleaned)
            .with_alphabet(&alphabet)
            .into_vec()
            .map_err(|e| Error::invalid_input(e.to_string()))
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        if input.is_empty() {
            return DetectCandidate {
                codec: "base58ripple".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            };
        }

        let valid = input.chars().filter(|c| RIPPLE_ALPHABET.contains(*c)).count();
        let ratio = valid as f32 / input.len() as f32;

        if ratio == 1.0 {
            DetectCandidate {
                codec: "base58ripple".to_string(),
                confidence: util::confidence::PARTIAL_MATCH,
                reasons: vec!["all chars valid".to_string()],
                warnings: vec![],
            }
        } else if ratio > 0.9 {
            DetectCandidate {
                codec: "base58ripple".to_string(),
                confidence: util::confidence::WEAK_MATCH,
                reasons: vec!["most chars valid".to_string()],
                warnings: vec![],
            }
        } else {
            DetectCandidate {
                codec: "base58ripple".to_string(),
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
    fn test_base58ripple_encode() {
        let codec = Base58Ripple;
        let encoded = codec.encode(b"hello").unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded.chars().all(|c| RIPPLE_ALPHABET.contains(c)));
    }

    #[test]
    fn test_base58ripple_decode() {
        let codec = Base58Ripple;
        let encoded = codec.encode(b"hello").unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base58ripple_roundtrip() {
        let codec = Base58Ripple;
        let test_cases = vec![
            b"test" as &[u8],
            b"Hello World",
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
    fn test_base58ripple_empty() {
        let codec = Base58Ripple;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_base58ripple_leading_zeros() {
        let codec = Base58Ripple;
        let data = vec![0, 0, 1, 2, 3];
        let encoded = codec.encode(&data).unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base58ripple_case_sensitive() {
        assert!(RIPPLE_ALPHABET.contains('r'));
        assert!(RIPPLE_ALPHABET.contains('R'));
        assert_ne!(RIPPLE_ALPHABET.find('r'), RIPPLE_ALPHABET.find('R'));
    }

    #[test]
    fn test_base58ripple_lenient_whitespace() {
        let codec = Base58Ripple;
        let encoded = codec.encode(b"test").unwrap();
        let with_spaces = format!("{} {}", &encoded[..2], &encoded[2..]);
        let decoded = codec.decode(&with_spaces, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"test");
    }

    #[test]
    fn test_base58ripple_detect() {
        let codec = Base58Ripple;
        let encoded = codec.encode(b"hello world").unwrap();
        assert!(codec.detect_score(&encoded).confidence > 0.4);
        assert!(codec.detect_score("hello$world").confidence < 0.1);
    }

    #[test]
    fn test_base58ripple_starts_with_r() {
        let codec = Base58Ripple;
        assert!(RIPPLE_ALPHABET.starts_with('r'));
        let encoded = codec.encode(&[0]).unwrap();
        assert_eq!(encoded, "r");
    }
}
