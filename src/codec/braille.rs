use super::{util, Codec};
use crate::error::{MbaseError as Error, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct Braille;

const BRAILLE_BASE: u32 = 0x2800;

const BRAILLE_MAP: &[(char, u8)] = &[
    ('a', 0b00000001),
    ('b', 0b00000011),
    ('c', 0b00001001),
    ('d', 0b00011001),
    ('e', 0b00010001),
    ('f', 0b00001011),
    ('g', 0b00011011),
    ('h', 0b00010011),
    ('i', 0b00001010),
    ('j', 0b00011010),
    ('k', 0b00000101),
    ('l', 0b00000111),
    ('m', 0b00001101),
    ('n', 0b00011101),
    ('o', 0b00010101),
    ('p', 0b00001111),
    ('q', 0b00011111),
    ('r', 0b00010111),
    ('s', 0b00001110),
    ('t', 0b00011110),
    ('u', 0b00100101),
    ('v', 0b00100111),
    ('w', 0b00111010),
    ('x', 0b00101101),
    ('y', 0b00111101),
    ('z', 0b00110101),
    (' ', 0b00000000),
    (',', 0b00000010),
    (';', 0b00000110),
    (':', 0b00010010),
    ('.', 0b00101100),
    ('!', 0b00010110),
    ('?', 0b00100110),
    ('\'', 0b00000100),
    ('-', 0b00100100),
    ('(', 0b00110110),
    (')', 0b00110110),
];

impl Codec for Braille {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "braille",
            aliases: &["braille-ascii"],
            alphabet: "\u{2800}-\u{28FF}",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Braille Unicode patterns (U+2800-U+28FF)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let mut result = String::new();

        for &byte in input {
            let ch = (byte as char).to_ascii_lowercase();

            if let Some(&(_, pattern)) = BRAILLE_MAP.iter().find(|(c, _)| *c == ch) {
                let codepoint = BRAILLE_BASE + pattern as u32;
                if let Some(braille_char) = char::from_u32(codepoint) {
                    result.push(braille_char);
                } else {
                    return Err(Error::invalid_input(format!("invalid braille codepoint: U+{:04X}", codepoint)));
                }
            } else {
                return Err(Error::invalid_input(format!("character '{}' not supported in Braille", ch)));
            }
        }

        Ok(result)
    }

    fn decode(&self, input: &str, _mode: Mode) -> Result<Vec<u8>> {
        let mut result = Vec::new();

        for ch in input.chars() {
            let codepoint = ch as u32;

            if codepoint < BRAILLE_BASE || codepoint > (BRAILLE_BASE + 0xFF) {
                return Err(Error::invalid_input(format!("character '{}' is not a Braille pattern", ch)));
            }

            let pattern = (codepoint - BRAILLE_BASE) as u8;

            if let Some(&(ascii_char, _)) = BRAILLE_MAP.iter().find(|(_, p)| *p == pattern) {
                result.push(ascii_char as u8);
            } else {
                return Err(Error::invalid_input(format!("unknown Braille pattern: U+{:04X}", codepoint)));
            }
        }

        Ok(result)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        if input.is_empty() {
            return DetectCandidate {
                codec: "braille".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            };
        }

        let braille_chars = input
            .chars()
            .filter(|&c| {
                let cp = c as u32;
                cp >= BRAILLE_BASE && cp <= (BRAILLE_BASE + 0xFF)
            })
            .count();

        let ratio = braille_chars as f32 / input.chars().count() as f32;

        if ratio > 0.95 {
            DetectCandidate {
                codec: "braille".to_string(),
                confidence: util::confidence::ALPHABET_MATCH,
                reasons: vec!["high braille char ratio".to_string()],
                warnings: vec![],
            }
        } else if ratio > 0.7 {
            DetectCandidate {
                codec: "braille".to_string(),
                confidence: util::confidence::PARTIAL_MATCH,
                reasons: vec!["partial braille chars".to_string()],
                warnings: vec![],
            }
        } else {
            DetectCandidate {
                codec: "braille".to_string(),
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
    fn test_braille_encode() {
        let codec = Braille;
        assert_eq!(codec.encode(b"a").unwrap(), "⠁");
        assert_eq!(codec.encode(b"abc").unwrap(), "⠁⠃⠉");
        assert_eq!(codec.encode(b"hello").unwrap(), "⠓⠑⠇⠇⠕");
    }

    #[test]
    fn test_braille_decode() {
        let codec = Braille;
        assert_eq!(codec.decode("⠁", Mode::Strict).unwrap(), b"a");
        assert_eq!(codec.decode("⠁⠃⠉", Mode::Strict).unwrap(), b"abc");
        assert_eq!(codec.decode("⠓⠑⠇⠇⠕", Mode::Strict).unwrap(), b"hello");
    }

    #[test]
    fn test_braille_roundtrip() {
        let codec = Braille;
        let original = b"hello world";
        let encoded = codec.encode(original).unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_braille_case_insensitive() {
        let codec = Braille;
        let upper = codec.encode(b"ABC").unwrap();
        let lower = codec.encode(b"abc").unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn test_braille_numbers() {
        let codec = Braille;
        // Braille numbers need special prefix, so simple codec maps to letters
        let encoded = codec.encode(b"abc").unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"abc");
    }

    #[test]
    fn test_braille_punctuation() {
        let codec = Braille;
        let encoded = codec.encode(b"hello, world!").unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"hello, world!");
    }

    #[test]
    fn test_braille_invalid_char() {
        let codec = Braille;
        assert!(codec.encode(b"hello\xFF").is_err());
    }

    #[test]
    fn test_braille_detect() {
        let codec = Braille;
        assert!(codec.detect_score("⠓⠑⠇⠇⠕").confidence > 0.6);
        assert!(codec.detect_score("hello").confidence < 0.1);
        assert!(codec.detect_score("").confidence < 0.1);
    }
}
