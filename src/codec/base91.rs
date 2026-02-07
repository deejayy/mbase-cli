use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const ALPHABET: &[u8; 91] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!#$%&()*+,./:;<=>?@[]^_`{|}~\"";

fn decode_table() -> [i8; 256] {
    let mut table = [-1i8; 256];
    for (i, &c) in ALPHABET.iter().enumerate() {
        table[c as usize] = i as i8;
    }
    table
}

pub struct Base91;

impl Codec for Base91 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base91",
            aliases: &["b91"],
            alphabet: std::str::from_utf8(ALPHABET).unwrap(),
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "basE91 encoding (highest density printable ASCII)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        if input.is_empty() {
            return Ok(String::new());
        }

        let mut result = Vec::with_capacity(input.len() * 16 / 13 + 2);
        let mut queue: u32 = 0;
        let mut nbits: u32 = 0;

        for &byte in input {
            queue |= (byte as u32) << nbits;
            nbits += 8;

            if nbits > 13 {
                let mut val = queue & 8191; // 13 bits
                if val > 88 {
                    queue >>= 13;
                    nbits -= 13;
                } else {
                    val = queue & 16383; // 14 bits
                    queue >>= 14;
                    nbits -= 14;
                }
                result.push(ALPHABET[(val % 91) as usize]);
                result.push(ALPHABET[(val / 91) as usize]);
            }
        }

        if nbits > 0 {
            result.push(ALPHABET[(queue % 91) as usize]);
            if nbits > 7 || queue > 90 {
                result.push(ALPHABET[(queue / 91) as usize]);
            }
        }

        Ok(String::from_utf8(result).unwrap())
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let table = decode_table();
        let input = if mode == Mode::Lenient {
            input.chars().filter(|c| !c.is_whitespace()).collect::<String>()
        } else {
            input.to_string()
        };

        if input.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(input.len() * 14 / 16);
        let mut queue: u32 = 0;
        let mut nbits: u32 = 0;
        let mut val: i32 = -1;

        for (pos, c) in input.chars().enumerate() {
            let d = table[c as usize];
            if d == -1 {
                return Err(MbaseError::InvalidCharacter { char: c, position: pos });
            }

            if val == -1 {
                val = d as i32;
            } else {
                val += (d as i32) * 91;
                queue |= (val as u32) << nbits;
                nbits += if (val & 8191) > 88 { 13 } else { 14 };

                while nbits > 7 {
                    result.push((queue & 255) as u8);
                    queue >>= 8;
                    nbits -= 8;
                }
                val = -1;
            }
        }

        if val != -1 {
            result.push((queue | ((val as u32) << nbits)) as u8);
        }

        Ok(result)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let table = decode_table();
        let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect();

        if clean.is_empty() {
            return DetectCandidate {
                codec: self.name().to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let invalid_count = clean.chars().filter(|&c| table[c as usize] == -1).count();
        if invalid_count > 0 {
            return DetectCandidate {
                codec: self.name().to_string(),
                confidence: 0.0,
                reasons: vec![format!("{} invalid characters", invalid_count)],
                warnings: vec![],
            };
        }

        let has_special = clean.chars().any(|c| "!#$%&()*+,./:;<=>?@[]^_`{|}~\"".contains(c));
        let confidence = if has_special {
            util::confidence::ALPHABET_MATCH
        } else {
            util::confidence::PARTIAL_MATCH
        };

        let mut reasons = vec!["all characters valid".to_string()];
        if has_special {
            reasons.push("contains base91-specific punctuation".to_string());
        }

        if self.decode(&clean, Mode::Lenient).is_ok() {
            reasons.push("decodes successfully".to_string());
        }

        DetectCandidate {
            codec: self.name().to_string(),
            confidence,
            reasons,
            warnings: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base91_empty() {
        let codec = Base91;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_base91_hello() {
        let codec = Base91;
        let encoded = codec.encode(b"Hello World!").unwrap();
        assert_eq!(encoded, ">OwJh>Io0Tv!8PE");
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Hello World!");
    }

    #[test]
    fn test_base91_roundtrip() {
        let codec = Base91;
        let inputs = [
            b"".to_vec(),
            b"a".to_vec(),
            b"ab".to_vec(),
            b"abc".to_vec(),
            b"Hello".to_vec(),
            b"The quick brown fox jumps over the lazy dog".to_vec(),
            (0..=255).collect::<Vec<u8>>(),
        ];
        for input in inputs {
            let encoded = codec.encode(&input).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, input, "roundtrip failed for {:?}", input);
        }
    }

    #[test]
    fn test_base91_invalid_char() {
        let codec = Base91;
        let result = codec.decode("Hello\x00World", Mode::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn test_base91_lenient_whitespace() {
        let codec = Base91;
        let encoded = codec.encode(b"test").unwrap();
        let with_space = format!("{} {}", &encoded[..2], &encoded[2..]);
        let decoded = codec.decode(&with_space, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"test");
    }

    #[test]
    fn test_base91_density() {
        let codec = Base91;
        let input: Vec<u8> = (0..=255).collect();
        let encoded = codec.encode(&input).unwrap();
        let ratio = encoded.len() as f64 / input.len() as f64;
        assert!(ratio < 1.25, "base91 should be ~23% overhead, got {:.2}%", (ratio - 1.0) * 100.0);
    }
}
