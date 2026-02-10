use super::{rfc1924, util, Codec};
use crate::error::{MbaseError as Error, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct Base85Chunked;

impl Codec for Base85Chunked {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base85chunked",
            aliases: &[],
            alphabet: rfc1924::RFC1924_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Base85 with chunked encoding (4-byte groups to 5-char groups)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        if input.is_empty() {
            return Ok(String::new());
        }

        let alphabet = rfc1924::RFC1924_ALPHABET.as_bytes();
        let mut result = String::new();

        for chunk in input.chunks(4) {
            let mut padded = [0u8; 4];
            padded[..chunk.len()].copy_from_slice(chunk);

            let val = ((padded[0] as u32) << 24) | ((padded[1] as u32) << 16) | ((padded[2] as u32) << 8) | (padded[3] as u32);

            let mut chars = [0u8; 5];
            let mut v = val;
            for i in (0..5).rev() {
                chars[i] = alphabet[(v % 85) as usize];
                v /= 85;
            }

            let output_len = match chunk.len() {
                1 => 2,
                2 => 3,
                3 => 4,
                4 => 5,
                _ => unreachable!(),
            };

            for &c in &chars[..output_len] {
                result.push(c as char);
            }
        }

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

        let mut result = Vec::new();
        let chars: Vec<char> = cleaned.chars().collect();

        let mut i = 0;
        while i < chars.len() {
            let chunk_len = std::cmp::min(5, chars.len() - i);
            let chunk = &chars[i..i + chunk_len];

            if chunk_len == 1 {
                return Err(Error::invalid_input("RFC1924 group cannot be single character"));
            }

            let mut val: u32 = 0;
            for (j, &c) in chunk.iter().enumerate() {
                let pos = i + j;
                let v = rfc1924::RFC1924_ALPHABET
                    .chars()
                    .position(|x| x == c)
                    .ok_or_else(|| Error::InvalidCharacter { char: c, position: pos })?;
                val = val * 85 + v as u32;
            }

            if chunk_len < 5 {
                for _ in chunk_len..5 {
                    val = val * 85 + 84;
                }
            }

            let bytes = [(val >> 24) as u8, (val >> 16) as u8, (val >> 8) as u8, val as u8];

            let output_len = match chunk_len {
                5 => 4,
                4 => 3,
                3 => 2,
                2 => 1,
                _ => unreachable!(),
            };

            result.extend_from_slice(&bytes[..output_len]);
            i += chunk_len;
        }

        Ok(result)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        if input.is_empty() {
            return DetectCandidate {
                codec: "base85chunked".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            };
        }

        let valid = input.chars().filter(|c| rfc1924::RFC1924_ALPHABET.contains(*c)).count();
        let ratio = valid as f32 / input.len() as f32;

        // Require 100% match and prefer length divisible by 5
        if ratio == 1.0 {
            if input.len() % 5 == 0 {
                DetectCandidate {
                    codec: "base85chunked".to_string(),
                    confidence: util::confidence::PARTIAL_MATCH,
                    reasons: vec!["all chars valid, length multiple of 5".to_string()],
                    warnings: vec![],
                }
            } else {
                DetectCandidate {
                    codec: "base85chunked".to_string(),
                    confidence: util::confidence::WEAK_MATCH,
                    reasons: vec!["all chars valid".to_string()],
                    warnings: vec![],
                }
            }
        } else {
            DetectCandidate {
                codec: "base85chunked".to_string(),
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
    fn test_base85chunked_encode() {
        let codec = Base85Chunked;
        let encoded = codec.encode(b"hello").unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded.chars().all(|c| rfc1924::RFC1924_ALPHABET.contains(c)));
    }

    #[test]
    fn test_base85chunked_decode() {
        let codec = Base85Chunked;
        let encoded = codec.encode(b"hello").unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base85chunked_roundtrip() {
        let codec = Base85Chunked;
        let test_cases = vec![
            b"test" as &[u8],
            b"Hello World",
            b"The quick brown fox",
            &[0, 1, 2, 3, 4, 5],
            &[255, 254, 253],
            &[0x86, 0x4F, 0xD2, 0x6F],
        ];

        for original in test_cases {
            let encoded = codec.encode(original).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, original, "roundtrip failed for {:?}", original);
        }
    }

    #[test]
    fn test_base85chunked_empty() {
        let codec = Base85Chunked;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_base85chunked_variable_length() {
        let codec = Base85Chunked;
        for len in 1..=10 {
            let data: Vec<u8> = (0..len).map(|i| (i * 17) as u8).collect();
            let encoded = codec.encode(&data).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, data, "roundtrip failed for length {}", len);
        }
    }

    #[test]
    fn test_base85chunked_single_byte() {
        let codec = Base85Chunked;
        let encoded = codec.encode(&[0x42]).unwrap();
        assert_eq!(encoded.len(), 2);
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, &[0x42]);
    }

    #[test]
    fn test_base85chunked_invalid_single_char() {
        let codec = Base85Chunked;
        assert!(codec.decode("A", Mode::Strict).is_err());
    }

    #[test]
    fn test_base85chunked_lenient_whitespace() {
        let codec = Base85Chunked;
        let encoded = codec.encode(b"test").unwrap();
        let with_spaces = format!("{} {}", &encoded[..3], &encoded[3..]);
        let decoded = codec.decode(&with_spaces, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"test");
    }

    #[test]
    fn test_base85chunked_case_sensitive() {
        assert!(rfc1924::RFC1924_ALPHABET.contains('A'));
        assert!(rfc1924::RFC1924_ALPHABET.contains('a'));
        assert_ne!(rfc1924::RFC1924_ALPHABET.find('A'), rfc1924::RFC1924_ALPHABET.find('a'));
    }

    #[test]
    fn test_base85chunked_detect() {
        let codec = Base85Chunked;
        let encoded = codec.encode(b"hello world test").unwrap();
        assert!(codec.detect_score(&encoded).confidence > 0.3);
        // Use a string with characters NOT in the alphabet (space and comma)
        assert!(codec.detect_score("hello, world").confidence < 0.1);
    }
}
