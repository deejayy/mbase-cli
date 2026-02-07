use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const CONSONANTS: &[u8; 16] = b"bdfghjklmnprstvz";
const VOWELS: &[u8; 4] = b"aiou";

fn consonant_index(c: char) -> Option<u8> {
    let c = c.to_ascii_lowercase();
    CONSONANTS.iter().position(|&x| x == c as u8).map(|i| i as u8)
}

fn vowel_index(c: char) -> Option<u8> {
    let c = c.to_ascii_lowercase();
    VOWELS.iter().position(|&x| x == c as u8).map(|i| i as u8)
}

pub struct Proquint;

impl Proquint {
    fn encode_u16(val: u16) -> String {
        let result = vec![
            CONSONANTS[((val >> 12) & 0x0F) as usize],
            VOWELS[((val >> 10) & 0x03) as usize],
            CONSONANTS[((val >> 6) & 0x0F) as usize],
            VOWELS[((val >> 4) & 0x03) as usize],
            CONSONANTS[(val & 0x0F) as usize],
        ];
        String::from_utf8(result).unwrap()
    }

    fn decode_quint(s: &str) -> Result<u16> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 5 {
            return Err(MbaseError::invalid_length(crate::error::LengthConstraint::Exact(5), chars.len()));
        }

        let c0 = consonant_index(chars[0]).ok_or_else(|| MbaseError::InvalidCharacter {
            char: chars[0],
            position: 0,
        })?;
        let v0 = vowel_index(chars[1]).ok_or_else(|| MbaseError::InvalidCharacter {
            char: chars[1],
            position: 1,
        })?;
        let c1 = consonant_index(chars[2]).ok_or_else(|| MbaseError::InvalidCharacter {
            char: chars[2],
            position: 2,
        })?;
        let v1 = vowel_index(chars[3]).ok_or_else(|| MbaseError::InvalidCharacter {
            char: chars[3],
            position: 3,
        })?;
        let c2 = consonant_index(chars[4]).ok_or_else(|| MbaseError::InvalidCharacter {
            char: chars[4],
            position: 4,
        })?;

        Ok(((c0 as u16) << 12) | ((v0 as u16) << 10) | ((c1 as u16) << 6) | ((v1 as u16) << 4) | (c2 as u16))
    }
}

impl Codec for Proquint {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "proquint",
            aliases: &["pq", "proq"],
            alphabet: "bdfghjklmnprstvz-aiou",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Proquint pronounceable identifiers (2 bytes per quint)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        if input.is_empty() {
            return Ok(String::new());
        }

        if !input.len().is_multiple_of(2) {
            return Err(MbaseError::invalid_length(crate::error::LengthConstraint::MultipleOf(2), input.len()));
        }

        let quints: Vec<String> = input
            .chunks(2)
            .map(|chunk| {
                let val = ((chunk[0] as u16) << 8) | (chunk[1] as u16);
                Self::encode_u16(val)
            })
            .collect();

        Ok(quints.join("-"))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let input = if mode == Mode::Lenient {
            input.chars().filter(|c| !c.is_whitespace() || *c == '-').collect::<String>()
        } else {
            input.to_string()
        };

        if input.is_empty() {
            return Ok(Vec::new());
        }

        let quints: Vec<&str> = input.split('-').filter(|s| !s.is_empty()).collect();
        let mut result = Vec::with_capacity(quints.len() * 2);

        for (idx, quint) in quints.iter().enumerate() {
            let val = Self::decode_quint(quint).map_err(|e| {
                if let MbaseError::InvalidCharacter { char: c, position: p } = e {
                    MbaseError::InvalidCharacter {
                        char: c,
                        position: idx * 6 + p,
                    }
                } else {
                    e
                }
            })?;
            result.push((val >> 8) as u8);
            result.push((val & 0xFF) as u8);
        }

        Ok(result)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect();

        if clean.is_empty() {
            return DetectCandidate {
                codec: self.name().to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let parts: Vec<&str> = clean.split('-').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return DetectCandidate {
                codec: self.name().to_string(),
                confidence: 0.0,
                reasons: vec!["no quints found".to_string()],
                warnings: vec![],
            };
        }

        let valid_quints = parts.iter().filter(|q| q.len() == 5).count();
        let all_valid_pattern = parts.iter().all(|q| {
            let chars: Vec<char> = q.chars().collect();
            chars.len() == 5
                && consonant_index(chars[0]).is_some()
                && vowel_index(chars[1]).is_some()
                && consonant_index(chars[2]).is_some()
                && vowel_index(chars[3]).is_some()
                && consonant_index(chars[4]).is_some()
        });

        if !all_valid_pattern {
            return DetectCandidate {
                codec: self.name().to_string(),
                confidence: 0.0,
                reasons: vec!["invalid quint pattern".to_string()],
                warnings: vec![],
            };
        }

        let has_separator = clean.contains('-');
        let confidence = if has_separator && valid_quints >= 2 {
            0.9
        } else if has_separator {
            util::confidence::ALPHABET_MATCH
        } else if valid_quints == 1 {
            util::confidence::PARTIAL_MATCH
        } else {
            util::confidence::WEAK_MATCH
        };

        DetectCandidate {
            codec: self.name().to_string(),
            confidence,
            reasons: vec![format!("{} valid quints", valid_quints), "CVCVC pattern matches".to_string()],
            warnings: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proquint_empty() {
        let codec = Proquint;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_proquint_known_vectors() {
        let codec = Proquint;
        // 127.0.0.1 = 0x7F000001
        assert_eq!(codec.encode(&[0x7F, 0x00, 0x00, 0x01]).unwrap(), "lusab-babad");
        assert_eq!(codec.decode("lusab-babad", Mode::Strict).unwrap(), &[0x7F, 0x00, 0x00, 0x01]);

        // 63.84.220.193 = 0x3F54DCC1
        assert_eq!(codec.encode(&[0x3F, 0x54, 0xDC, 0xC1]).unwrap(), "gutih-tugad");
        assert_eq!(codec.decode("gutih-tugad", Mode::Strict).unwrap(), &[0x3F, 0x54, 0xDC, 0xC1]);
    }

    #[test]
    fn test_proquint_roundtrip() {
        let codec = Proquint;
        let inputs: Vec<Vec<u8>> = vec![
            vec![0, 0],
            vec![255, 255],
            vec![0, 0, 0, 0],
            vec![1, 2, 3, 4],
            vec![0xDE, 0xAD, 0xBE, 0xEF],
        ];
        for input in inputs {
            let encoded = codec.encode(&input).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, input);
        }
    }

    #[test]
    fn test_proquint_case_insensitive() {
        let codec = Proquint;
        let lower = codec.decode("lusab-babad", Mode::Strict).unwrap();
        let upper = codec.decode("LUSAB-BABAD", Mode::Strict).unwrap();
        let mixed = codec.decode("LuSaB-BaBaD", Mode::Strict).unwrap();
        assert_eq!(lower, upper);
        assert_eq!(lower, mixed);
    }

    #[test]
    fn test_proquint_odd_length_error() {
        let codec = Proquint;
        let result = codec.encode(&[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_proquint_invalid_char() {
        let codec = Proquint;
        let result = codec.decode("lusab-bpx", Mode::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn test_proquint_detect() {
        let codec = Proquint;
        let score = codec.detect_score("lusab-babad");
        assert!(score.confidence >= 0.7);

        let score = codec.detect_score("hello");
        assert!(score.confidence < 0.5);
    }
}
