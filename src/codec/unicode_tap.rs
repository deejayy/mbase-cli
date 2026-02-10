use super::Codec;
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct UnicodeCodepoints;

impl Codec for UnicodeCodepoints {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "unicode",
            aliases: &["codepoints", "u+"],
            alphabet: "U+0123456789ABCDEFabcdef ",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Unicode code points (U+XXXX format)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let text = String::from_utf8_lossy(input);
        let codepoints: Vec<String> = text.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        Ok(codepoints.join(" "))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = if mode == Mode::Lenient {
            input.replace(&['\t', '\n', '\r'], " ")
        } else {
            input.to_string()
        };

        let parts: Vec<&str> = cleaned.split_whitespace().collect();
        let mut result = String::new();

        for part in parts {
            let hex_str = if part.starts_with("U+") || part.starts_with("u+") {
                &part[2..]
            } else if part.starts_with("\\u") {
                &part[2..]
            } else if part.starts_with("0x") || part.starts_with("0X") {
                &part[2..]
            } else {
                part
            };

            let codepoint = u32::from_str_radix(hex_str, 16).map_err(|_| MbaseError::invalid_input(format!("invalid hex: {}", part)))?;

            let ch = char::from_u32(codepoint).ok_or_else(|| MbaseError::invalid_input(format!("invalid codepoint: U+{:X}", codepoint)))?;

            result.push(ch);
        }

        Ok(result.into_bytes())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        let warnings = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "unicode".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let u_plus_count = input.matches("U+").count() + input.matches("u+").count();

        if u_plus_count > 0 {
            let parts: Vec<&str> = input.split_whitespace().collect();
            let valid_count = parts
                .iter()
                .filter(|p| {
                    if let Some(hex) = p.strip_prefix("U+").or_else(|| p.strip_prefix("u+")) {
                        u32::from_str_radix(hex, 16).is_ok()
                    } else {
                        false
                    }
                })
                .count();

            if valid_count == parts.len() && valid_count > 0 {
                confidence = 0.9;
                reasons.push(format!("all {} tokens are valid U+XXXX format", valid_count));
            } else if valid_count > 0 {
                confidence = 0.6;
                reasons.push(format!("{}/{} tokens valid", valid_count, parts.len()));
            }
        }

        DetectCandidate {
            codec: "unicode".to_string(),
            confidence,
            reasons,
            warnings,
        }
    }
}

pub struct TapCode;

impl Codec for TapCode {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "tapcode",
            aliases: &["tap", "knock"],
            alphabet: "12345 ",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Tap code (Polybius square as digit pairs)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let text = String::from_utf8_lossy(input).to_uppercase();
        let codes: Vec<String> = text
            .chars()
            .filter_map(|c| {
                let pos = match c {
                    'A'..='Z' => {
                        let mut p = c as u8 - b'A';
                        if c >= 'K' {
                            // K maps to same position as C (position 2)
                            if c == 'K' {
                                p = 2;
                            } else {
                                // L-Z: subtract 1 because K is skipped
                                p -= 1;
                            }
                        }
                        Some(p)
                    }
                    ' ' => return Some("  ".to_string()),
                    _ => None,
                }?;
                let row = pos / 5 + 1;
                let col = pos % 5 + 1;
                Some(format!("{}{}", row, col))
            })
            .collect();

        if codes.is_empty() {
            return Err(MbaseError::invalid_input("no encodable characters found"));
        }

        Ok(codes.join(" "))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = if mode == Mode::Lenient {
            input.trim().to_string()
        } else {
            input.to_string()
        };

        let mut result = String::new();

        for pair in cleaned.split_whitespace() {
            if pair.is_empty() {
                continue;
            }

            if pair.len() != 2 {
                return Err(MbaseError::invalid_input(format!("invalid tap code pair: {}", pair)));
            }

            let row = pair
                .chars()
                .nth(0)
                .unwrap()
                .to_digit(10)
                .ok_or_else(|| MbaseError::invalid_input(format!("invalid row digit: {}", pair)))?;
            let col = pair
                .chars()
                .nth(1)
                .unwrap()
                .to_digit(10)
                .ok_or_else(|| MbaseError::invalid_input(format!("invalid col digit: {}", pair)))?;

            if row < 1 || row > 5 || col < 1 || col > 5 {
                return Err(MbaseError::invalid_input(format!("coordinates out of range: {}", pair)));
            }

            let pos = (row - 1) * 5 + (col - 1);

            // Tap code grid: A-J (pos 0-9), then L-Z (pos 10-24)
            // K shares position 2 with C
            let ch = if pos == 2 {
                'C' // C/K share this position, decode as C
            } else if pos < 10 {
                (b'A' + pos as u8) as char
            } else {
                // For pos >= 10, we're in L-Z range, add 1 to skip K
                (b'A' + pos as u8 + 1) as char
            };

            result.push(ch);
        }

        Ok(result.into_bytes())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        let warnings = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "tapcode".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let valid_pairs = parts
            .iter()
            .filter(|p| p.len() == 2 && p.chars().all(|c| c >= '1' && c <= '5'))
            .count();

        if valid_pairs == parts.len() && valid_pairs > 0 {
            confidence = 0.8;
            reasons.push(format!("all {} tokens are valid tap code pairs (11-55)", valid_pairs));
        } else if valid_pairs > parts.len() / 2 {
            confidence = 0.4;
            reasons.push(format!("{}/{} tokens valid", valid_pairs, parts.len()));
        }

        DetectCandidate {
            codec: "tapcode".to_string(),
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
    fn test_unicode_encode() {
        assert_eq!(UnicodeCodepoints.encode(b"A").unwrap(), "U+0041");
        assert_eq!(UnicodeCodepoints.encode(b"Hello").unwrap(), "U+0048 U+0065 U+006C U+006C U+006F");
        assert_eq!(UnicodeCodepoints.encode("🦀".as_bytes()).unwrap(), "U+1F980");
    }

    #[test]
    fn test_unicode_decode() {
        assert_eq!(UnicodeCodepoints.decode("U+0041", Mode::Strict).unwrap(), b"A");
        assert_eq!(
            UnicodeCodepoints
                .decode("U+0048 U+0065 U+006C U+006C U+006F", Mode::Strict)
                .unwrap(),
            b"Hello"
        );
    }

    #[test]
    fn test_unicode_roundtrip() {
        let data = b"Hello, World!";
        let encoded = UnicodeCodepoints.encode(data).unwrap();
        assert_eq!(UnicodeCodepoints.decode(&encoded, Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_unicode_case_insensitive() {
        assert_eq!(UnicodeCodepoints.decode("u+0041", Mode::Lenient).unwrap(), b"A");
        assert_eq!(UnicodeCodepoints.decode("U+0041", Mode::Lenient).unwrap(), b"A");
    }

    #[test]
    fn test_unicode_alternative_formats() {
        assert_eq!(UnicodeCodepoints.decode("0x0041", Mode::Lenient).unwrap(), b"A");
        assert_eq!(UnicodeCodepoints.decode("41", Mode::Lenient).unwrap(), b"A");
    }

    #[test]
    fn test_unicode_emoji() {
        let crab = "🦀";
        let encoded = UnicodeCodepoints.encode(crab.as_bytes()).unwrap();
        let decoded = UnicodeCodepoints.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), crab);
    }

    #[test]
    fn test_tapcode_encode() {
        assert_eq!(TapCode.encode(b"HELLO").unwrap(), "23 15 31 31 34");
        assert_eq!(TapCode.encode(b"ABC").unwrap(), "11 12 13");
    }

    #[test]
    fn test_tapcode_decode() {
        assert_eq!(TapCode.decode("23 15 31 31 34", Mode::Strict).unwrap(), b"HELLO");
        assert_eq!(TapCode.decode("11 12 13", Mode::Strict).unwrap(), b"ABC");
    }

    #[test]
    fn test_tapcode_roundtrip() {
        let data = b"THEQUICK";
        let encoded = TapCode.encode(data).unwrap();
        let decoded = TapCode.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"THEQUICC"); // K becomes C
    }

    #[test]
    fn test_tapcode_case_insensitive() {
        let upper = TapCode.encode(b"HELLO").unwrap();
        let lower = TapCode.encode(b"hello").unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn test_tapcode_grid() {
        // Test first row
        assert_eq!(TapCode.encode(b"A").unwrap(), "11");
        assert_eq!(TapCode.encode(b"B").unwrap(), "12");
        // C and K share position
        assert_eq!(TapCode.encode(b"C").unwrap(), "13");
        assert_eq!(TapCode.encode(b"K").unwrap(), "13");
        assert_eq!(TapCode.encode(b"D").unwrap(), "14");
        assert_eq!(TapCode.encode(b"E").unwrap(), "15");
    }

    #[test]
    fn test_tapcode_invalid_coords() {
        assert!(TapCode.decode("16", Mode::Strict).is_err());
        assert!(TapCode.decode("61", Mode::Strict).is_err());
        assert!(TapCode.decode("00", Mode::Strict).is_err());
    }
}
