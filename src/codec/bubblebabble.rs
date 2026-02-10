use super::{util, Codec};
use crate::error::{MbaseError as Error, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct BubbleBabble;

const VOWELS: &[u8; 6] = b"aeiouy";
const CONSONANTS: &[u8; 17] = b"bcdfghklmnprstvzx";

fn vowel_index(c: char) -> Option<u8> {
    let c = c.to_ascii_lowercase();
    VOWELS.iter().position(|&x| x == c as u8).map(|i| i as u8)
}

fn consonant_index(c: char) -> Option<u8> {
    let c = c.to_ascii_lowercase();
    CONSONANTS.iter().position(|&x| x == c as u8).map(|i| i as u8)
}

impl Codec for BubbleBabble {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "bubblebabble",
            aliases: &["bubble", "babble"],
            alphabet: "aeiouy-bcdfghklmnprstvzx",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Bubble Babble pronounceable encoding (OpenSSH fingerprint style)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let mut result = String::from("x");
        let mut checksum = 1u32;

        let mut i = 0;
        while i < input.len() {
            if i + 1 < input.len() {
                let byte1 = input[i] as u32;
                let byte2 = input[i + 1] as u32;

                if i > 0 {
                    result.push('-');
                }

                result.push(VOWELS[((((byte1 >> 6) & 3) + checksum) % 6) as usize] as char);
                result.push(CONSONANTS[(byte1 >> 2) as usize & 0x0F] as char);
                result.push(VOWELS[(((byte1 & 3) + (checksum / 6)) % 6) as usize] as char);
                result.push(CONSONANTS[(byte2 >> 4) as usize & 0x0F] as char);
                result.push(CONSONANTS[(byte2 & 0x0F) as usize] as char);

                checksum = ((checksum * 5) + (byte1 * 7) + byte2) % 36;
                i += 2;
            } else {
                if i > 0 {
                    result.push('-');
                }
                let byte = input[i] as u32;
                result.push(VOWELS[((((byte >> 6) & 3) + checksum) % 6) as usize] as char);
                result.push(CONSONANTS[(byte >> 2) as usize & 0x0F] as char);
                result.push(VOWELS[(((byte & 3) + (checksum / 6)) % 6) as usize] as char);
                i += 1;
            }
        }

        if !input.is_empty() {
            result.push('-');
        }
        result.push(VOWELS[(checksum % 6) as usize] as char);
        result.push('x');

        Ok(result)
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = if mode == Mode::Lenient {
            input.chars().filter(|c| !c.is_whitespace()).collect::<String>()
        } else {
            input.to_string()
        };

        if cleaned.is_empty() {
            return Ok(Vec::new());
        }

        let cleaned_lower = cleaned.to_lowercase();

        if !cleaned_lower.starts_with('x') || !cleaned_lower.ends_with('x') {
            return Err(Error::invalid_input("Bubble Babble must start and end with 'x'"));
        }

        let core = &cleaned_lower[1..cleaned_lower.len() - 1];
        if core.is_empty() {
            return Ok(Vec::new());
        }

        let tuples: Vec<&str> = core.split('-').collect();
        let mut result = Vec::new();
        let mut checksum = 1u32;

        for (idx, tuple) in tuples.iter().enumerate() {
            let chars: Vec<char> = tuple.chars().collect();

            if chars.len() == 5 {
                let v1 = vowel_index(chars[0]).ok_or_else(|| Error::InvalidCharacter {
                    char: chars[0],
                    position: idx * 6,
                })?;
                let c1 = consonant_index(chars[1]).ok_or_else(|| Error::InvalidCharacter {
                    char: chars[1],
                    position: idx * 6 + 1,
                })?;
                let v2 = vowel_index(chars[2]).ok_or_else(|| Error::InvalidCharacter {
                    char: chars[2],
                    position: idx * 6 + 2,
                })?;
                let c2 = consonant_index(chars[3]).ok_or_else(|| Error::InvalidCharacter {
                    char: chars[3],
                    position: idx * 6 + 3,
                })?;
                let c3 = consonant_index(chars[4]).ok_or_else(|| Error::InvalidCharacter {
                    char: chars[4],
                    position: idx * 6 + 4,
                })?;

                let high_bits = ((v1 as u32 + 36 - checksum) % 6) << 6;
                let byte1 = (high_bits | ((c1 as u32) << 2) | ((v2 as u32 + 36 - (checksum / 6)) % 6)) as u8;
                let byte2 = (((c2 as u32) << 4) | c3 as u32) as u8;

                result.push(byte1);
                result.push(byte2);

                checksum = ((checksum * 5) + (byte1 as u32 * 7) + byte2 as u32) % 36;
            } else if chars.len() == 3 {
                let v1 = vowel_index(chars[0]).ok_or_else(|| Error::InvalidCharacter {
                    char: chars[0],
                    position: idx * 6,
                })?;
                let c1 = consonant_index(chars[1]).ok_or_else(|| Error::InvalidCharacter {
                    char: chars[1],
                    position: idx * 6 + 1,
                })?;
                let v2 = vowel_index(chars[2]).ok_or_else(|| Error::InvalidCharacter {
                    char: chars[2],
                    position: idx * 6 + 2,
                })?;

                let high_bits = ((v1 as u32 + 36 - checksum) % 6) << 6;
                let byte = (high_bits | ((c1 as u32) << 2) | ((v2 as u32 + 36 - (checksum / 6)) % 6)) as u8;

                result.push(byte);
            } else if chars.len() == 1 {
                continue;
            } else {
                return Err(Error::invalid_input(format!("invalid tuple length: {}", chars.len())));
            }
        }

        Ok(result)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect();

        if clean.is_empty() {
            return DetectCandidate {
                codec: "bubblebabble".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            };
        }

        let lower = clean.to_lowercase();
        if !lower.starts_with('x') || !lower.ends_with('x') {
            return DetectCandidate {
                codec: "bubblebabble".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            };
        }

        let has_dashes = clean.contains('-');
        let valid_chars = clean
            .chars()
            .filter(|c| {
                VOWELS.contains(&(*c as u8).to_ascii_lowercase())
                    || CONSONANTS.contains(&(*c as u8).to_ascii_lowercase())
                    || *c == '-'
                    || *c == 'x'
            })
            .count();

        let ratio = valid_chars as f32 / clean.len() as f32;

        if ratio > 0.95 && has_dashes && lower.starts_with('x') && lower.ends_with('x') {
            DetectCandidate {
                codec: "bubblebabble".to_string(),
                confidence: util::confidence::ALPHABET_MATCH,
                reasons: vec!["valid bubble babble format".to_string()],
                warnings: vec![],
            }
        } else if ratio > 0.9 && lower.starts_with('x') {
            DetectCandidate {
                codec: "bubblebabble".to_string(),
                confidence: util::confidence::PARTIAL_MATCH,
                reasons: vec!["partial match".to_string()],
                warnings: vec![],
            }
        } else {
            DetectCandidate {
                codec: "bubblebabble".to_string(),
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
    fn test_bubblebabble_encode() {
        let codec = BubbleBabble;
        let encoded = codec.encode(b"test").unwrap();
        assert!(encoded.starts_with('x'));
        assert!(encoded.ends_with('x'));
        assert!(encoded.contains('-'));
    }

    #[test]
    fn test_bubblebabble_decode() {
        let codec = BubbleBabble;
        let encoded = codec.encode(b"test").unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"test");
    }

    #[test]
    fn test_bubblebabble_roundtrip() {
        let codec = BubbleBabble;
        let test_cases = vec![
            b"" as &[u8],
            b"a",
            b"ab",
            b"test",
            b"hello",
            b"Hello World",
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
    fn test_bubblebabble_empty() {
        let codec = BubbleBabble;
        let encoded = codec.encode(b"").unwrap();
        assert_eq!(encoded, "xex");
        assert_eq!(codec.decode("xex", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_bubblebabble_case_insensitive() {
        let codec = BubbleBabble;
        let data = b"test";
        let encoded = codec.encode(data).unwrap();
        let upper = encoded.to_uppercase();

        let decoded_lower = codec.decode(&encoded, Mode::Strict).unwrap();
        let decoded_upper = codec.decode(&upper, Mode::Strict).unwrap();
        assert_eq!(decoded_lower, decoded_upper);
        assert_eq!(decoded_lower, data);
    }

    #[test]
    fn test_bubblebabble_invalid_no_x_wrapper() {
        let codec = BubbleBabble;
        assert!(codec.decode("hello", Mode::Strict).is_err());
        assert!(codec.decode("xhello", Mode::Strict).is_err());
        assert!(codec.decode("hellox", Mode::Strict).is_err());
    }

    #[test]
    fn test_bubblebabble_lenient_whitespace() {
        let codec = BubbleBabble;
        let encoded = codec.encode(b"test").unwrap();
        let with_spaces = encoded
            .chars()
            .collect::<Vec<_>>()
            .chunks(3)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ");
        let decoded = codec.decode(&with_spaces, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"test");
    }

    #[test]
    fn test_bubblebabble_detect() {
        let codec = BubbleBabble;
        let encoded = codec.encode(b"hello world").unwrap();
        assert!(codec.detect_score(&encoded).confidence > 0.6);
        assert!(codec.detect_score("hello").confidence < 0.1);
        assert!(codec.detect_score("xhello").confidence < 0.6);
    }

    #[test]
    fn test_bubblebabble_single_byte() {
        let codec = BubbleBabble;
        let encoded = codec.encode(&[0x42]).unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, &[0x42]);
    }
}
