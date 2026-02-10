use super::Codec;
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct A1Z26;

impl Codec for A1Z26 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "a1z26",
            aliases: &["letternum", "alphanumeric"],
            alphabet: "0123456789-",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Letter position encoding (A=1, B=2, ..., Z=26)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let text = String::from_utf8_lossy(input).to_uppercase();
        let numbers: Vec<String> = text
            .chars()
            .filter_map(|c| {
                if c >= 'A' && c <= 'Z' {
                    Some((c as u8 - b'A' + 1).to_string())
                } else if c == ' ' {
                    Some("0".to_string())
                } else {
                    None
                }
            })
            .collect();

        if numbers.is_empty() {
            return Err(MbaseError::invalid_input("no encodable letters found"));
        }

        Ok(numbers.join("-"))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = if mode == Mode::Lenient {
            input.replace(&[' ', '\t', '\n', '\r'], "")
        } else {
            input.to_string()
        };

        let parts: Vec<&str> = cleaned.split('-').collect();
        let mut result = String::new();

        for part in parts {
            if part.is_empty() {
                continue;
            }

            let num: u8 = part
                .parse()
                .map_err(|_| MbaseError::invalid_input(format!("invalid number: {}", part)))?;

            if num == 0 {
                result.push(' ');
            } else if num >= 1 && num <= 26 {
                result.push((b'A' + num - 1) as char);
            } else {
                return Err(MbaseError::invalid_input(format!("number out of range (1-26): {}", num)));
            }
        }

        Ok(result.into_bytes())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        let warnings = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "a1z26".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let dash_count = input.matches('-').count();
        let digit_count = input.chars().filter(|c| c.is_ascii_digit()).count();
        let _total_chars = input.len();

        if dash_count > 0 && digit_count > 0 {
            let parts: Vec<&str> = input.split('-').collect();
            let valid_parts = parts
                .iter()
                .filter(|p| if let Ok(num) = p.parse::<u8>() { num <= 26 } else { false })
                .count();

            if valid_parts == parts.len() {
                confidence = 0.7;
                reasons.push(format!("all {} numbers in range 0-26", parts.len()));
            } else if valid_parts > parts.len() / 2 {
                confidence = 0.4;
                reasons.push(format!("{}/{} numbers valid", valid_parts, parts.len()));
            }
        }

        DetectCandidate {
            codec: "a1z26".to_string(),
            confidence,
            reasons,
            warnings,
        }
    }
}

pub struct Rot18;

impl Codec for Rot18 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "rot18",
            aliases: &["rot-18"],
            alphabet: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "ROT13 for letters + ROT5 for digits",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(input
            .iter()
            .map(|&b| {
                let c = b as char;
                match c {
                    'A'..='Z' => (((c as u8 - b'A') + 13) % 26 + b'A') as char,
                    'a'..='z' => (((c as u8 - b'a') + 13) % 26 + b'a') as char,
                    '0'..='9' => (((c as u8 - b'0') + 5) % 10 + b'0') as char,
                    _ => c,
                }
            })
            .collect())
    }

    fn decode(&self, input: &str, _mode: Mode) -> Result<Vec<u8>> {
        Ok(input
            .chars()
            .map(|c| match c {
                'A'..='Z' => ((c as u8 - b'A') + 13) % 26 + b'A',
                'a'..='z' => ((c as u8 - b'a') + 13) % 26 + b'a',
                '0'..='9' => ((c as u8 - b'0') + 5) % 10 + b'0',
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
                codec: "rot18".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let alnum_count = input.chars().filter(|c| c.is_ascii_alphanumeric()).count();
        let alnum_ratio = alnum_count as f64 / input.len() as f64;

        if alnum_ratio > 0.5 {
            confidence = 0.2;
            reasons.push("contains alphanumeric characters".to_string());
            warnings.push("ROT18 is ambiguous without context".to_string());
        }

        DetectCandidate {
            codec: "rot18".to_string(),
            confidence,
            reasons,
            warnings: vec!["ROT18 is ambiguous without context".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a1z26_encode() {
        assert_eq!(A1Z26.encode(b"HELLO").unwrap(), "8-5-12-12-15");
        assert_eq!(A1Z26.encode(b"ABC").unwrap(), "1-2-3");
        assert_eq!(A1Z26.encode(b"XYZ").unwrap(), "24-25-26");
    }

    #[test]
    fn test_a1z26_decode() {
        assert_eq!(A1Z26.decode("8-5-12-12-15", Mode::Strict).unwrap(), b"HELLO");
        assert_eq!(A1Z26.decode("1-2-3", Mode::Strict).unwrap(), b"ABC");
        assert_eq!(A1Z26.decode("24-25-26", Mode::Strict).unwrap(), b"XYZ");
    }

    #[test]
    fn test_a1z26_roundtrip() {
        let data = b"THEQUICKBROWNFOX";
        let encoded = A1Z26.encode(data).unwrap();
        assert_eq!(A1Z26.decode(&encoded, Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_a1z26_with_space() {
        assert_eq!(A1Z26.encode(b"HELLO WORLD").unwrap(), "8-5-12-12-15-0-23-15-18-12-4");
        assert_eq!(A1Z26.decode("8-5-12-12-15-0-23-15-18-12-4", Mode::Strict).unwrap(), b"HELLO WORLD");
    }

    #[test]
    fn test_a1z26_case_insensitive() {
        let upper = A1Z26.encode(b"HELLO").unwrap();
        let lower = A1Z26.encode(b"hello").unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn test_a1z26_invalid_number() {
        assert!(A1Z26.decode("27", Mode::Strict).is_err());
        assert!(A1Z26.decode("1-27-3", Mode::Strict).is_err());
    }

    #[test]
    fn test_rot18_encode() {
        assert_eq!(Rot18.encode(b"Hello123").unwrap(), "Uryyb678");
        assert_eq!(Rot18.encode(b"Test5").unwrap(), "Grfg0");
        assert_eq!(Rot18.encode(b"ABC789").unwrap(), "NOP234");
    }

    #[test]
    fn test_rot18_decode() {
        assert_eq!(Rot18.decode("Uryyb678", Mode::Strict).unwrap(), b"Hello123");
        assert_eq!(Rot18.decode("Grfg0", Mode::Strict).unwrap(), b"Test5");
    }

    #[test]
    fn test_rot18_roundtrip() {
        let data = b"Hello World 123!";
        let encoded = Rot18.encode(data).unwrap();
        assert_eq!(Rot18.decode(&encoded, Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_rot18_symmetric() {
        let encoded = Rot18.encode(b"Test5").unwrap();
        let double_encoded = Rot18.encode(encoded.as_bytes()).unwrap();
        assert_eq!(double_encoded, "Test5");
    }

    #[test]
    fn test_rot18_digits() {
        assert_eq!(Rot18.encode(b"0123456789").unwrap(), "5678901234");
    }
}
