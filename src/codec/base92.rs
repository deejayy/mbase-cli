use super::{util, Codec};
use crate::error::{MbaseError as Error, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct Base92;

const BASE92_ALPHABET: &str = "!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~";

impl Codec for Base92 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base92",
            aliases: &[],
            alphabet: BASE92_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Base92 (92 printable ASCII characters)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        if input.is_empty() {
            return Ok(String::new());
        }

        let alphabet = BASE92_ALPHABET.as_bytes();

        // Handle leading zeros separately
        let leading_zeros = input.iter().take_while(|&&x| x == 0).count();

        if leading_zeros == input.len() {
            // All zeros
            return Ok((alphabet[0] as char).to_string().repeat(input.len()));
        }

        // Use Vec<u8> as bigint for non-zero part
        let mut num: Vec<u8> = input[leading_zeros..].to_vec();

        let mut result = Vec::new();
        while !num.iter().all(|&x| x == 0) {
            let mut remainder = 0u16;
            for byte in num.iter_mut() {
                let temp = (remainder as u16 * 256) + *byte as u16;
                *byte = (temp / 92) as u8;
                remainder = temp % 92;
            }
            result.push(alphabet[remainder as usize] as char);

            // Remove leading zeros
            while num.first() == Some(&0) && num.len() > 1 {
                num.remove(0);
            }
        }

        result.reverse();

        // Prepend encoded leading zeros
        for _ in 0..leading_zeros {
            result.insert(0, alphabet[0] as char);
        }

        Ok(result.into_iter().collect())
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

        let alphabet = BASE92_ALPHABET.as_bytes();
        // Count leading zeros (first char of alphabet)
        let first_char = alphabet[0] as char;
        let leading_zeros = cleaned.chars().take_while(|&c| c == first_char).count();

        // If all characters are zeros, return that many zero bytes
        if leading_zeros == cleaned.len() {
            return Ok(vec![0; leading_zeros]);
        }

        let mut num: Vec<u8> = vec![0];

        for (i, c) in cleaned.chars().skip(leading_zeros).enumerate() {
            let val = BASE92_ALPHABET.find(c).ok_or_else(|| Error::InvalidCharacter {
                char: c,
                position: i + leading_zeros,
            })? as u16;

            // Multiply num by 92 and add val
            let mut carry = val;
            for byte in num.iter_mut().rev() {
                let temp = (*byte as u16) * 92 + carry;
                *byte = (temp % 256) as u8;
                carry = temp / 256;
            }
            while carry > 0 {
                num.insert(0, (carry % 256) as u8);
                carry /= 256;
            }
        }

        // Prepend leading zero bytes
        for _ in 0..leading_zeros {
            num.insert(0, 0);
        }

        Ok(num)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        if input.is_empty() {
            return DetectCandidate {
                codec: "base92".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            };
        }

        let valid = input.chars().filter(|c| BASE92_ALPHABET.contains(*c)).count();
        let ratio = valid as f32 / input.len() as f32;

        // Require 100% match - any invalid character means 0 confidence
        if ratio == 1.0 {
            DetectCandidate {
                codec: "base92".to_string(),
                confidence: util::confidence::PARTIAL_MATCH,
                reasons: vec!["all characters in alphabet".to_string()],
                warnings: vec![],
            }
        } else {
            DetectCandidate {
                codec: "base92".to_string(),
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
    fn test_base92_encode() {
        let codec = Base92;
        let encoded = codec.encode(b"hello").unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded.chars().all(|c| BASE92_ALPHABET.contains(c)));
    }

    #[test]
    fn test_base92_decode() {
        let codec = Base92;
        let encoded = codec.encode(b"hello").unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base92_roundtrip() {
        let codec = Base92;
        let test_cases = vec![
            b"test" as &[u8],
            b"Hello World",
            b"The quick brown fox jumps over the lazy dog",
            &[0, 1, 2, 3, 4, 5],
            &[255, 254, 253],
            &[0x00],
            &[0xFF],
        ];

        for original in test_cases {
            let encoded = codec.encode(original).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, original, "roundtrip failed for {:?}", original);
        }
    }

    #[test]
    fn test_base92_empty() {
        let codec = Base92;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_base92_single_byte() {
        let codec = Base92;
        for i in 0..=255u8 {
            let data = vec![i];
            let encoded = codec.encode(&data).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, data, "roundtrip failed for byte {}", i);
        }
    }

    #[test]
    fn test_base92_case_sensitive() {
        assert!(BASE92_ALPHABET.contains('A'));
        assert!(BASE92_ALPHABET.contains('a'));
        assert_ne!(BASE92_ALPHABET.find('A'), BASE92_ALPHABET.find('a'));
    }

    #[test]
    fn test_base92_lenient_whitespace() {
        let codec = Base92;
        let encoded = codec.encode(b"test").unwrap();
        let with_spaces = format!("{} {}", &encoded[..2], &encoded[2..]);
        let decoded = codec.decode(&with_spaces, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"test");
    }

    #[test]
    fn test_base92_detect() {
        let codec = Base92;
        let encoded = codec.encode(b"hello world").unwrap();
        assert!(codec.detect_score(&encoded).confidence > 0.4);
        assert!(codec.detect_score("hello world").confidence < 0.1);
    }

    #[test]
    fn test_base92_alphabet_size() {
        assert_eq!(BASE92_ALPHABET.len(), 92);
        let unique: std::collections::HashSet<_> = BASE92_ALPHABET.chars().collect();
        assert_eq!(unique.len(), 92);
    }
}
