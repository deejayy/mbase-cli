use super::Codec;
use crate::error::Result;
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct Atbash;

impl Codec for Atbash {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "atbash",
            aliases: &[],
            alphabet: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Atbash cipher (A↔Z, B↔Y, etc.)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(input
            .iter()
            .map(|&b| {
                let c = b as char;
                match c {
                    'A'..='Z' => (b'Z' - (c as u8 - b'A')) as char,
                    'a'..='z' => (b'z' - (c as u8 - b'a')) as char,
                    _ => c,
                }
            })
            .collect())
    }

    fn decode(&self, input: &str, _mode: Mode) -> Result<Vec<u8>> {
        Ok(input
            .chars()
            .map(|c| match c {
                'A'..='Z' => b'Z' - (c as u8 - b'A'),
                'a'..='z' => b'z' - (c as u8 - b'a'),
                _ => c as u8,
            })
            .collect())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "atbash".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let alpha_count = input.chars().filter(|c| c.is_ascii_alphabetic()).count();
        let alpha_ratio = alpha_count as f64 / input.len() as f64;

        if alpha_ratio > 0.5 {
            confidence = 0.15;
            reasons.push("contains alphabetic characters".to_string());
            warnings.push("Atbash is ambiguous without context".to_string());
        }

        DetectCandidate {
            codec: "atbash".to_string(),
            confidence,
            reasons,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atbash_encode() {
        assert_eq!(Atbash.encode(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ").unwrap(), "ZYXWVUTSRQPONMLKJIHGFEDCBA");
        assert_eq!(Atbash.encode(b"abcdefghijklmnopqrstuvwxyz").unwrap(), "zyxwvutsrqponmlkjihgfedcba");
        assert_eq!(Atbash.encode(b"Hello").unwrap(), "Svool");
        assert_eq!(Atbash.encode(b"HELLO").unwrap(), "SVOOL");
    }

    #[test]
    fn test_atbash_decode() {
        assert_eq!(Atbash.decode("Svool", Mode::Strict).unwrap(), b"Hello");
        assert_eq!(Atbash.decode("SVOOL", Mode::Strict).unwrap(), b"HELLO");
    }

    #[test]
    fn test_atbash_roundtrip() {
        let data = b"The Quick Brown Fox";
        let encoded = Atbash.encode(data).unwrap();
        assert_eq!(Atbash.decode(&encoded, Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_atbash_symmetric() {
        let encoded = Atbash.encode(b"test").unwrap();
        let double_encoded = Atbash.encode(encoded.as_bytes()).unwrap();
        assert_eq!(double_encoded, "test");
    }

    #[test]
    fn test_atbash_non_alpha() {
        assert_eq!(Atbash.encode(b"Hello, World! 123").unwrap(), "Svool, Dliow! 123");
    }

    #[test]
    fn test_atbash_empty() {
        assert_eq!(Atbash.encode(&[]).unwrap(), "");
        assert_eq!(Atbash.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_atbash_mixed_case() {
        assert_eq!(Atbash.encode(b"HeLLo").unwrap(), "SvOOl");
    }
}
