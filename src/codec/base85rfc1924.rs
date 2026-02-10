use super::{rfc1924, util, Codec};
use crate::error::Result;
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct Base85Rfc1924;

impl Codec for Base85Rfc1924 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base85rfc1924",
            aliases: &["rfc1924"],
            alphabet: rfc1924::RFC1924_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Base85 RFC1924 (128-bit big-integer encoding)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let num = rfc1924::bytes_to_u128(input)?;
        Ok(rfc1924::encode_u128(num))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = if mode == Mode::Lenient {
            input.chars().filter(|c| !c.is_whitespace()).collect::<String>()
        } else {
            input.to_string()
        };

        let num = rfc1924::decode_u128(&cleaned)?;
        Ok(rfc1924::u128_to_bytes(num).to_vec())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        if input.len() != rfc1924::RFC1924_ENCODED_LEN {
            return DetectCandidate {
                codec: "base85rfc1924".to_string(),
                confidence: 0.0,
                reasons: vec![format!("length must be exactly {} characters", rfc1924::RFC1924_ENCODED_LEN)],
                warnings: vec![],
            };
        }

        let valid = input.chars().filter(|c| rfc1924::RFC1924_ALPHABET.contains(*c)).count();

        if valid == rfc1924::RFC1924_ENCODED_LEN {
            DetectCandidate {
                codec: "base85rfc1924".to_string(),
                confidence: util::confidence::ALPHABET_MATCH,
                reasons: vec![format!(
                    "exactly {} chars, all valid RFC1924 alphabet",
                    rfc1924::RFC1924_ENCODED_LEN
                )],
                warnings: vec![],
            }
        } else {
            DetectCandidate {
                codec: "base85rfc1924".to_string(),
                confidence: 0.0,
                reasons: vec!["contains invalid characters".to_string()],
                warnings: vec![],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base85rfc1924_spec_example() {
        let codec = Base85Rfc1924;
        let input = [
            0x10, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x08, 0x00, 0x20, 0x0C, 0x41, 0x7A,
        ];
        let encoded = codec.encode(&input).unwrap();
        assert_eq!(encoded, "4)+k&C#VzJ4br>0wv%Yp");

        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_base85rfc1924_roundtrip() {
        let codec = Base85Rfc1924;
        let test_cases = vec![
            vec![0u8; 16],
            vec![255u8; 16],
            vec![
                0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
            ],
        ];

        for original in test_cases {
            let encoded = codec.encode(&original).unwrap();
            assert_eq!(encoded.len(), rfc1924::RFC1924_ENCODED_LEN);
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, original, "roundtrip failed for {:?}", original);
        }
    }

    #[test]
    fn test_base85rfc1924_wrong_input_length() {
        let codec = Base85Rfc1924;
        assert!(codec.encode(&[1, 2, 3]).is_err());
        assert!(codec.encode(&[0u8; 15]).is_err());
        assert!(codec.encode(&[0u8; 17]).is_err());
    }

    #[test]
    fn test_base85rfc1924_wrong_decode_length() {
        let codec = Base85Rfc1924;
        assert!(codec.decode("too_short", Mode::Strict).is_err());
        assert!(codec.decode("way_too_long_for_rfc1924", Mode::Strict).is_err());
    }

    #[test]
    fn test_base85rfc1924_invalid_char() {
        let codec = Base85Rfc1924;
        let with_invalid = "1234567890123456789,";
        assert!(codec.decode(with_invalid, Mode::Strict).is_err());
    }

    #[test]
    fn test_base85rfc1924_lenient_whitespace() {
        let codec = Base85Rfc1924;
        let input = [
            0x10, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x08, 0x00, 0x20, 0x0C, 0x41, 0x7A,
        ];
        let encoded = codec.encode(&input).unwrap();
        let with_space = format!("{} {}", &encoded[..10], &encoded[10..]);
        let decoded = codec.decode(&with_space, Mode::Lenient).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_base85rfc1924_detect() {
        let codec = Base85Rfc1924;
        let input = [
            0x10, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x08, 0x00, 0x20, 0x0C, 0x41, 0x7A,
        ];
        let encoded = codec.encode(&input).unwrap();
        assert!(codec.detect_score(&encoded).confidence > 0.6);
        assert_eq!(codec.detect_score("wrong_length").confidence, 0.0);
        assert_eq!(codec.detect_score("12345678901234567890").confidence, 0.7);
        assert_eq!(codec.detect_score("1234567890123456789,").confidence, 0.0);
    }

    #[test]
    fn test_base85rfc1924_all_zeros() {
        let codec = Base85Rfc1924;
        let input = [0u8; 16];
        let encoded = codec.encode(&input).unwrap();
        assert_eq!(encoded, "00000000000000000000");
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_base85rfc1924_all_ones() {
        let codec = Base85Rfc1924;
        let input = [0xFFu8; 16];
        let encoded = codec.encode(&input).unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, input);
    }
}
