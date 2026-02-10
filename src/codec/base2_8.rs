use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct Base2;

impl Codec for Base2 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base2",
            aliases: &["binary", "bin"],
            alphabet: "01",
            multibase_code: Some('0'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Binary representation (base2)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(input.iter().map(|&b| format!("{:08b}", b)).collect::<String>())
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);

        if cleaned.len() % 8 != 0 {
            return Err(MbaseError::invalid_length(crate::error::LengthConstraint::MultipleOf(8), cleaned.len()));
        }

        cleaned
            .as_bytes()
            .chunks(8)
            .map(|chunk| {
                let s = std::str::from_utf8(chunk).map_err(|_| MbaseError::invalid_input("invalid UTF-8"))?;
                u8::from_str_radix(s, 2).map_err(|e| MbaseError::invalid_input(format!("invalid binary digit: {}", e)))
            })
            .collect()
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "base2".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let binary_count = input.chars().filter(|c| *c == '0' || *c == '1').count();
        let ratio = binary_count as f64 / input.len() as f64;

        if ratio == 1.0 {
            confidence = util::confidence::ALPHABET_MATCH;
            reasons.push("all characters are binary digits".to_string());

            if input.len() >= 16 && input.len() % 8 == 0 {
                confidence = util::confidence::ALPHABET_MATCH;
                reasons.push("length is multiple of 8".to_string());
            } else if input.len() % 8 != 0 {
                warnings.push("length not multiple of 8".to_string());
            }
        } else if ratio > 0.9 {
            confidence = util::confidence::WEAK_MATCH;
            warnings.push(format!("{:.1}% non-binary characters", (1.0 - ratio) * 100.0));
        }

        DetectCandidate {
            codec: "base2".to_string(),
            confidence,
            reasons,
            warnings,
        }
    }
}

pub struct Base8;

impl Codec for Base8 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base8",
            aliases: &["octal", "oct"],
            alphabet: "01234567",
            multibase_code: Some('7'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Octal representation (base8)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(input.iter().map(|&b| format!("{:03o}", b)).collect::<String>())
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);

        if cleaned.len() % 3 != 0 {
            return Err(MbaseError::invalid_length(crate::error::LengthConstraint::MultipleOf(3), cleaned.len()));
        }

        cleaned
            .as_bytes()
            .chunks(3)
            .map(|chunk| {
                let s = std::str::from_utf8(chunk).map_err(|_| MbaseError::invalid_input("invalid UTF-8"))?;
                u8::from_str_radix(s, 8).map_err(|e| MbaseError::invalid_input(format!("invalid octal digit: {}", e)))
            })
            .collect()
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "base8".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let octal_count = input.chars().filter(|c| ('0'..='7').contains(c)).count();
        let ratio = octal_count as f64 / input.len() as f64;

        if ratio == 1.0 {
            confidence = util::confidence::ALPHABET_MATCH;
            reasons.push("all characters are octal digits".to_string());

            if input.len() % 3 == 0 {
                confidence = util::confidence::ALPHABET_MATCH;
                reasons.push("length is multiple of 3".to_string());
            } else {
                warnings.push("length not multiple of 3".to_string());
            }
        } else if ratio > 0.9 {
            confidence = util::confidence::WEAK_MATCH;
            warnings.push(format!("{:.1}% non-octal characters", (1.0 - ratio) * 100.0));
        }

        DetectCandidate {
            codec: "base8".to_string(),
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
    fn test_base2_encode() {
        assert_eq!(Base2.encode(b"A").unwrap(), "01000001");
        assert_eq!(Base2.encode(b"Hello").unwrap(), "0100100001100101011011000110110001101111");
        assert_eq!(Base2.encode(&[0, 255]).unwrap(), "0000000011111111");
    }

    #[test]
    fn test_base2_decode() {
        assert_eq!(Base2.decode("01000001", Mode::Strict).unwrap(), b"A");
        assert_eq!(Base2.decode("0100100001100101011011000110110001101111", Mode::Strict).unwrap(), b"Hello");
    }

    #[test]
    fn test_base2_roundtrip() {
        let data = b"\x00\x7f\x80\xff";
        let encoded = Base2.encode(data).unwrap();
        assert_eq!(Base2.decode(&encoded, Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_base2_invalid_length() {
        assert!(Base2.decode("0100000", Mode::Strict).is_err());
    }

    #[test]
    fn test_base2_invalid_digit() {
        assert!(Base2.decode("01000002", Mode::Strict).is_err());
    }

    #[test]
    fn test_base2_empty() {
        assert_eq!(Base2.encode(&[]).unwrap(), "");
        assert_eq!(Base2.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_base8_encode() {
        assert_eq!(Base8.encode(b"A").unwrap(), "101");
        assert_eq!(Base8.encode(b"Hello").unwrap(), "110145154154157");
        assert_eq!(Base8.encode(&[0, 255, 64]).unwrap(), "000377100");
    }

    #[test]
    fn test_base8_decode() {
        assert_eq!(Base8.decode("101", Mode::Strict).unwrap(), b"A");
        assert_eq!(Base8.decode("110145154154157", Mode::Strict).unwrap(), b"Hello");
    }

    #[test]
    fn test_base8_roundtrip() {
        let data = b"\x00\x7f\x80\xff";
        let encoded = Base8.encode(data).unwrap();
        assert_eq!(Base8.decode(&encoded, Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_base8_invalid_length() {
        assert!(Base8.decode("10", Mode::Strict).is_err());
    }

    #[test]
    fn test_base8_invalid_digit() {
        assert!(Base8.decode("108", Mode::Strict).is_err());
    }

    #[test]
    fn test_base8_empty() {
        assert_eq!(Base8.encode(&[]).unwrap(), "");
        assert_eq!(Base8.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_base8_all_bytes() {
        for i in 0..=255u8 {
            let encoded = Base8.encode(&[i]).unwrap();
            assert_eq!(Base8.decode(&encoded, Mode::Strict).unwrap(), vec![i]);
        }
    }
}
