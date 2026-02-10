use super::{util, Codec};
use crate::error::{MbaseError as Error, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};
use data_encoding::{Encoding, Specification};
use std::sync::OnceLock;

pub struct Base32WordSafe;

const WORDSAFE_ALPHABET: &str = "ybndrfg8ejkmcpqxot1uwisza345h769";

static BASE32_WORDSAFE: OnceLock<Encoding> = OnceLock::new();

fn get_wordsafe_encoding() -> &'static Encoding {
    BASE32_WORDSAFE.get_or_init(|| {
        let mut spec = Specification::new();
        spec.symbols.push_str(WORDSAFE_ALPHABET);
        // Make it case-insensitive by translating uppercase to lowercase
        spec.translate.from.push_str(&WORDSAFE_ALPHABET.to_uppercase());
        spec.translate.to.push_str(WORDSAFE_ALPHABET);
        spec.encoding().unwrap()
    })
}

impl Codec for Base32WordSafe {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base32wordsafe",
            aliases: &["base32ws"],
            alphabet: WORDSAFE_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Base32 WordSafe (human-friendly, avoids similar chars)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(get_wordsafe_encoding().encode(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = if mode == Mode::Lenient {
            input.chars().filter(|c| !c.is_whitespace()).collect::<String>()
        } else {
            input.to_string()
        };

        get_wordsafe_encoding()
            .decode(cleaned.as_bytes())
            .map_err(|e| Error::invalid_input(e.to_string()))
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        if input.is_empty() {
            return DetectCandidate {
                codec: "base32wordsafe".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            };
        }

        let valid_chars = input.chars().filter(|c| WORDSAFE_ALPHABET.contains(*c)).count();
        let ratio = valid_chars as f32 / input.len() as f32;

        if ratio > 0.95 {
            DetectCandidate {
                codec: "base32wordsafe".to_string(),
                confidence: util::confidence::ALPHABET_MATCH,
                reasons: vec!["high match ratio".to_string()],
                warnings: vec![],
            }
        } else if ratio > 0.8 {
            DetectCandidate {
                codec: "base32wordsafe".to_string(),
                confidence: util::confidence::PARTIAL_MATCH,
                reasons: vec!["partial match".to_string()],
                warnings: vec![],
            }
        } else {
            DetectCandidate {
                codec: "base32wordsafe".to_string(),
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
    fn test_base32wordsafe_encode() {
        let codec = Base32WordSafe;
        let encoded = codec.encode(b"hello").unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded.chars().all(|c| WORDSAFE_ALPHABET.contains(c)));
    }

    #[test]
    fn test_base32wordsafe_decode() {
        let codec = Base32WordSafe;
        let encoded = codec.encode(b"hello").unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base32wordsafe_roundtrip() {
        let codec = Base32WordSafe;
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
    fn test_base32wordsafe_case_insensitive() {
        let codec = Base32WordSafe;
        let data = b"test";
        let encoded_lower = codec.encode(data).unwrap();
        let encoded_upper = encoded_lower.to_uppercase();

        let decoded_lower = codec.decode(&encoded_lower, Mode::Lenient).unwrap();
        let decoded_upper = codec.decode(&encoded_upper, Mode::Lenient).unwrap();
        assert_eq!(decoded_lower, decoded_upper);
    }

    #[test]
    fn test_base32wordsafe_empty() {
        let codec = Base32WordSafe;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_base32wordsafe_invalid_char() {
        let codec = Base32WordSafe;
        assert!(codec.decode("ABC01", Mode::Strict).is_err());
    }

    #[test]
    fn test_base32wordsafe_lenient_whitespace() {
        let codec = Base32WordSafe;
        let encoded = codec.encode(b"test").unwrap();
        let with_spaces = format!("{} {}", &encoded[..4], &encoded[4..]);
        let decoded = codec.decode(&with_spaces, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"test");
    }

    #[test]
    fn test_base32wordsafe_detect() {
        let codec = Base32WordSafe;
        let encoded = codec.encode(b"hello world").unwrap();
        assert!(codec.detect_score(&encoded).confidence > 0.6);
        assert!(codec.detect_score("ABCDEFGHIJKLMNOP").confidence < 0.1);
    }

    #[test]
    fn test_base32wordsafe_no_ambiguous_chars() {
        // z-base-32 avoids: 0 (zero), l (lowercase L), v (confused with u), and 2 (confused with z)
        assert!(!WORDSAFE_ALPHABET.contains('0'));
        assert!(!WORDSAFE_ALPHABET.contains('l'));
        assert!(!WORDSAFE_ALPHABET.contains('v'));
        assert!(!WORDSAFE_ALPHABET.contains('2'));
        // But it does include 1, which is distinct from i/I in the font used
        assert!(WORDSAFE_ALPHABET.contains('1'));
    }
}
