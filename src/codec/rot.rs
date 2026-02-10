use super::Codec;
use crate::error::Result;
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct Rot13;

impl Codec for Rot13 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "rot13",
            aliases: &["rot-13"],
            alphabet: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "ROT13 letter substitution (A-Z rotated by 13)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(input
            .iter()
            .map(|&b| {
                let c = b as char;
                match c {
                    'A'..='Z' => ((((c as u8 - b'A') + 13) % 26) + b'A') as char,
                    'a'..='z' => ((((c as u8 - b'a') + 13) % 26) + b'a') as char,
                    _ => c,
                }
            })
            .collect())
    }

    fn decode(&self, input: &str, _mode: Mode) -> Result<Vec<u8>> {
        Ok(input
            .chars()
            .map(|c| match c {
                'A'..='Z' => (((c as u8 - b'A') + 13) % 26) + b'A',
                'a'..='z' => (((c as u8 - b'a') + 13) % 26) + b'a',
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
                codec: "rot13".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let alpha_count = input.chars().filter(|c| c.is_ascii_alphabetic()).count();
        let alpha_ratio = alpha_count as f64 / input.len() as f64;

        if alpha_ratio > 0.5 {
            confidence = 0.2;
            reasons.push("contains alphabetic characters".to_string());
            warnings.push("ROT13 is ambiguous without context".to_string());
        }

        DetectCandidate {
            codec: "rot13".to_string(),
            confidence,
            reasons,
            warnings,
        }
    }
}

pub struct Rot47;

impl Codec for Rot47 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "rot47",
            aliases: &["rot-47"],
            alphabet: "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "ROT47 extended ASCII substitution (!-~ rotated by 47)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(input
            .iter()
            .map(|&b| {
                let c = b as char;
                if c >= '!' && c <= '~' {
                    let shifted = (c as u8 - b'!' + 47) % 94 + b'!';
                    shifted as char
                } else {
                    c
                }
            })
            .collect())
    }

    fn decode(&self, input: &str, _mode: Mode) -> Result<Vec<u8>> {
        Ok(input
            .chars()
            .map(|c| {
                if c >= '!' && c <= '~' {
                    (c as u8 - b'!' + 47) % 94 + b'!'
                } else {
                    c as u8
                }
            })
            .collect())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "rot47".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let printable_count = input.chars().filter(|c| *c >= '!' && *c <= '~').count();
        let printable_ratio = printable_count as f64 / input.len() as f64;

        if printable_ratio > 0.8 {
            confidence = 0.2;
            reasons.push("contains printable ASCII characters".to_string());
            warnings.push("ROT47 is ambiguous without context".to_string());
        }

        DetectCandidate {
            codec: "rot47".to_string(),
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
    fn test_rot13_encode() {
        assert_eq!(Rot13.encode(b"Hello").unwrap(), "Uryyb");
        assert_eq!(Rot13.encode(b"HELLO").unwrap(), "URYYB");
        assert_eq!(Rot13.encode(b"hello").unwrap(), "uryyb");
        assert_eq!(
            Rot13.encode(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz").unwrap(),
            "NOPQRSTUVWXYZABCDEFGHIJKLMnopqrstuvwxyzabcdefghijklm"
        );
    }

    #[test]
    fn test_rot13_decode() {
        assert_eq!(Rot13.decode("Uryyb", Mode::Strict).unwrap(), b"Hello");
        assert_eq!(Rot13.decode("URYYB", Mode::Strict).unwrap(), b"HELLO");
    }

    #[test]
    fn test_rot13_roundtrip() {
        let data = b"The Quick Brown Fox Jumps Over The Lazy Dog!";
        let encoded = Rot13.encode(data).unwrap();
        assert_eq!(Rot13.decode(&encoded, Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_rot13_symmetric() {
        let encoded = Rot13.encode(b"test").unwrap();
        let double_encoded = Rot13.encode(encoded.as_bytes()).unwrap();
        assert_eq!(double_encoded, "test");
    }

    #[test]
    fn test_rot13_non_alpha() {
        assert_eq!(Rot13.encode(b"Hello, World! 123").unwrap(), "Uryyb, Jbeyq! 123");
    }

    #[test]
    fn test_rot47_encode() {
        assert_eq!(Rot47.encode(b"Hello").unwrap(), "w6==@");
        assert_eq!(Rot47.encode(b"The Quick Brown Fox Jumps Over The Lazy Dog.").unwrap(), "%96 \"F:4< qC@H? u@I yF>AD ~G6C %96 {2KJ s@8]");
    }

    #[test]
    fn test_rot47_decode() {
        assert_eq!(Rot47.decode("w6==@", Mode::Strict).unwrap(), b"Hello");
    }

    #[test]
    fn test_rot47_roundtrip() {
        let data = b"Hello, World! 123 @#$%";
        let encoded = Rot47.encode(data).unwrap();
        assert_eq!(Rot47.decode(&encoded, Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_rot47_symmetric() {
        let encoded = Rot47.encode(b"test").unwrap();
        let double_encoded = Rot47.encode(encoded.as_bytes()).unwrap();
        assert_eq!(double_encoded, "test");
    }

    #[test]
    fn test_rot47_numbers() {
        assert_eq!(Rot47.encode(b"0123456789").unwrap(), "_`abcdefgh");
    }

    #[test]
    fn test_rot47_special_chars() {
        assert_eq!(Rot47.encode(b"!@#$%").unwrap(), "PoRST");
    }
}
