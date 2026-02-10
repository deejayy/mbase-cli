use super::{util, Codec};
use crate::error::{MbaseError as Error, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};
use std::collections::HashMap;

pub struct Baudot;

const BAUDOT_LETTERS: [char; 32] = [
    '\0', 'E', '\n', 'A', ' ', 'S', 'I', 'U', '\r', 'D', 'R', 'J', 'N', 'F', 'C', 'K', 'T', 'Z', 'L', 'W', 'H', 'Y', 'P', 'Q', 'O', 'B',
    'G', '\0', 'M', 'X', 'V', '\0',
];

const BAUDOT_FIGURES: [char; 32] = [
    '\0', '3', '\n', '-', ' ', '\'', '8', '7', '\r', '$', '4', '\u{0007}', ',', '!', ':', '(', '5', '"', ')', '2', '#', '6', '0', '1', '9',
    '?', '&', '\0', '.', '/', ';', '\0',
];

const LTRS_CODE: u8 = 0x1F;
const FIGS_CODE: u8 = 0x1B;

impl Codec for Baudot {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "baudot",
            aliases: &["ita2", "baudot-ita2"],
            alphabet: "01",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Baudot code (ITA2 5-bit telegraph encoding)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let mut result = Vec::new();
        let mut in_letters = true;

        let letter_map: HashMap<char, u8> = BAUDOT_LETTERS
            .iter()
            .enumerate()
            .filter(|(_, &c)| c != '\0')
            .map(|(i, &c)| (c, i as u8))
            .collect();

        let figure_map: HashMap<char, u8> = BAUDOT_FIGURES
            .iter()
            .enumerate()
            .filter(|(_, &c)| c != '\0')
            .map(|(i, &c)| (c, i as u8))
            .collect();

        for &byte in input {
            let ch = (byte as char).to_uppercase().next().unwrap_or(byte as char);

            if let Some(&code) = letter_map.get(&ch) {
                if !in_letters {
                    result.extend_from_slice(&format!("{:05b}", LTRS_CODE).as_bytes());
                    in_letters = true;
                }
                result.extend_from_slice(&format!("{:05b}", code).as_bytes());
            } else if let Some(&code) = figure_map.get(&ch) {
                if in_letters {
                    result.extend_from_slice(&format!("{:05b}", FIGS_CODE).as_bytes());
                    in_letters = false;
                }
                result.extend_from_slice(&format!("{:05b}", code).as_bytes());
            } else {
                return Err(Error::invalid_input(format!("character '{}' not supported in Baudot", ch)));
            }
        }

        Ok(String::from_utf8(result).unwrap())
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = if mode == Mode::Lenient {
            input.chars().filter(|c| *c == '0' || *c == '1').collect::<String>()
        } else {
            input.to_string()
        };

        if cleaned.len() % 5 != 0 {
            return Err(Error::invalid_input("Baudot input length must be multiple of 5"));
        }

        let mut result = Vec::new();
        let mut in_letters = true;

        for chunk in cleaned.as_bytes().chunks(5) {
            let binary_str = std::str::from_utf8(chunk).map_err(|_| Error::invalid_input("invalid UTF-8 in binary string"))?;
            let code = u8::from_str_radix(binary_str, 2).map_err(|_| Error::invalid_input("invalid binary digits"))?;

            if code == LTRS_CODE {
                in_letters = true;
                continue;
            } else if code == FIGS_CODE {
                in_letters = false;
                continue;
            }

            let ch = if in_letters {
                BAUDOT_LETTERS[code as usize]
            } else {
                BAUDOT_FIGURES[code as usize]
            };

            if ch == '\0' {
                return Err(Error::invalid_input(format!("invalid Baudot code: {:05b}", code)));
            }

            result.push(ch as u8);
        }

        Ok(result)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let cleaned = input.chars().filter(|c| *c == '0' || *c == '1').collect::<String>();

        if cleaned.is_empty() || cleaned.len() % 5 != 0 {
            return DetectCandidate {
                codec: "baudot".to_string(),
                confidence: 0.0,
                reasons: vec!["empty or invalid length".to_string()],
                warnings: vec![],
            };
        }

        let binary_chars = input.chars().filter(|c| *c == '0' || *c == '1').count();
        let ratio = binary_chars as f32 / input.len() as f32;

        if ratio < 0.9 {
            return DetectCandidate {
                codec: "baudot".to_string(),
                confidence: 0.0,
                reasons: vec!["low binary ratio".to_string()],
                warnings: vec![],
            };
        }

        let valid_chunks = cleaned
            .as_bytes()
            .chunks(5)
            .filter(|chunk| {
                if let Ok(s) = std::str::from_utf8(chunk) {
                    if let Ok(code) = u8::from_str_radix(s, 2) {
                        return code < 32;
                    }
                }
                false
            })
            .count();

        let total_chunks = cleaned.len() / 5;
        if valid_chunks == total_chunks {
            DetectCandidate {
                codec: "baudot".to_string(),
                confidence: util::confidence::PARTIAL_MATCH,
                reasons: vec!["valid baudot codes".to_string()],
                warnings: vec![],
            }
        } else {
            DetectCandidate {
                codec: "baudot".to_string(),
                confidence: 0.0,
                reasons: vec!["invalid codes".to_string()],
                warnings: vec![],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baudot_encode() {
        let codec = Baudot;
        // H=10100, E=00001, L=10010, L=10010, O=11000
        assert_eq!(codec.encode(b"HELLO").unwrap(), "1010000001100101001011000");
        // A=00011
        assert_eq!(codec.encode(b"A").unwrap(), "00011");
        // T=10000, E=00001, S=00101, T=10000
        assert_eq!(codec.encode(b"TEST").unwrap(), "10000000010010110000");
    }

    #[test]
    fn test_baudot_decode() {
        let codec = Baudot;
        assert_eq!(codec.decode("00011", Mode::Strict).unwrap(), b"A");
        assert_eq!(codec.decode("1010000001100101001011000", Mode::Strict).unwrap(), b"HELLO");
    }

    #[test]
    fn test_baudot_with_figures() {
        let codec = Baudot;
        let encoded = codec.encode(b"A1").unwrap();
        assert_eq!(codec.decode(&encoded, Mode::Strict).unwrap(), b"A1");
    }

    #[test]
    fn test_baudot_roundtrip() {
        let codec = Baudot;
        let original = b"THE QUICK BROWN FOX";
        let encoded = codec.encode(original).unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_baudot_case_insensitive() {
        let codec = Baudot;
        let upper = codec.encode(b"HELLO").unwrap();
        let lower = codec.encode(b"hello").unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn test_baudot_invalid_length() {
        let codec = Baudot;
        assert!(codec.decode("0001", Mode::Strict).is_err());
    }

    #[test]
    fn test_baudot_lenient_mode() {
        let codec = Baudot;
        let result = codec.decode("00011 00011", Mode::Lenient).unwrap();
        assert_eq!(result, b"AA");
    }

    #[test]
    fn test_baudot_detect() {
        let codec = Baudot;
        assert!(codec.detect_score("1010000001100101001011000").confidence > 0.4);
        assert!(codec.detect_score("not binary").confidence < 0.1);
        assert!(codec.detect_score("0001").confidence < 0.1);
    }
}
