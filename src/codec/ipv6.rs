use super::{rfc1924, util, Codec};
use crate::error::{MbaseError as Error, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};
use std::net::Ipv6Addr;
use std::str::FromStr;

pub struct Ipv6;

impl Codec for Ipv6 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "ipv6",
            aliases: &["ipv6-rfc1924"],
            alphabet: rfc1924::RFC1924_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "IPv6 RFC1924 compact representation (128-bit as base85)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let input_str = std::str::from_utf8(input).map_err(|_| Error::invalid_input("input must be valid UTF-8 IPv6 address string"))?;

        let addr = Ipv6Addr::from_str(input_str.trim()).map_err(|e| Error::invalid_input(format!("invalid IPv6 address: {}", e)))?;

        let bytes = addr.octets();
        let num = rfc1924::bytes_to_u128(&bytes)?;
        Ok(rfc1924::encode_u128(num))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = if mode == Mode::Lenient {
            input.chars().filter(|c| !c.is_whitespace()).collect::<String>()
        } else {
            input.to_string()
        };

        let num = rfc1924::decode_u128(&cleaned)?;
        let bytes = rfc1924::u128_to_bytes(num);
        let addr = Ipv6Addr::from(bytes);
        Ok(addr.to_string().into_bytes())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        if input.len() != rfc1924::RFC1924_ENCODED_LEN {
            return DetectCandidate {
                codec: "ipv6".to_string(),
                confidence: 0.0,
                reasons: vec![format!("length must be exactly {} characters", rfc1924::RFC1924_ENCODED_LEN)],
                warnings: vec![],
            };
        }

        let valid = input.chars().filter(|c| rfc1924::RFC1924_ALPHABET.contains(*c)).count();

        if valid == rfc1924::RFC1924_ENCODED_LEN {
            DetectCandidate {
                codec: "ipv6".to_string(),
                confidence: util::confidence::ALPHABET_MATCH,
                reasons: vec![format!(
                    "exactly {} chars, all valid RFC1924 alphabet",
                    rfc1924::RFC1924_ENCODED_LEN
                )],
                warnings: vec![],
            }
        } else {
            DetectCandidate {
                codec: "ipv6".to_string(),
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
    fn test_ipv6_rfc1924_example1() {
        let codec = Ipv6;
        let encoded = codec.encode(b"1080:0:0:0:8:800:200C:417A").unwrap();
        assert_eq!(encoded, "4)+k&C#VzJ4br>0wv%Yp");

        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), "1080::8:800:200c:417a");
    }

    #[test]
    fn test_ipv6_rfc1924_example2() {
        let codec = Ipv6;
        let encoded = codec.encode(b"FEDC:BA98:7654:3210:FEDC:BA98:7654:3210").unwrap();
        assert_eq!(encoded.len(), 20);

        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), "fedc:ba98:7654:3210:fedc:ba98:7654:3210");
    }

    #[test]
    fn test_ipv6_unspecified() {
        let codec = Ipv6;
        let encoded = codec.encode(b"::").unwrap();
        assert_eq!(encoded.len(), 20);

        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), "::");
    }

    #[test]
    fn test_ipv6_loopback() {
        let codec = Ipv6;
        let encoded = codec.encode(b"::1").unwrap();
        assert_eq!(encoded.len(), 20);

        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), "::1");
    }

    #[test]
    fn test_ipv6_wrong_input() {
        let codec = Ipv6;
        assert!(codec.encode(b"not an ipv6").is_err());
        assert!(codec.encode(b"256.256.256.256").is_err());
        assert!(codec.encode(&[0xFF, 0xAB, 0xCD]).is_err());
    }

    #[test]
    fn test_ipv6_wrong_length_decode() {
        let codec = Ipv6;
        assert!(codec.decode("too_short", Mode::Strict).is_err());
        assert!(codec.decode("way_too_long_for_ipv6_encoding", Mode::Strict).is_err());
    }

    #[test]
    fn test_ipv6_invalid_char() {
        let codec = Ipv6;
        let with_invalid = "1234567890123456789,";
        assert!(codec.decode(with_invalid, Mode::Strict).is_err());
    }

    #[test]
    fn test_ipv6_lenient_whitespace() {
        let codec = Ipv6;
        let encoded = codec.encode(b"::1").unwrap();
        let with_space = format!("{} {}", &encoded[..10], &encoded[10..]);
        let decoded = codec.decode(&with_space, Mode::Lenient).unwrap();
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), "::1");
    }

    #[test]
    fn test_ipv6_detect() {
        let codec = Ipv6;
        let encoded = codec.encode(b"1080::8:800:200C:417A").unwrap();
        assert!(codec.detect_score(&encoded).confidence > 0.6);
        assert_eq!(codec.detect_score("wrong_length").confidence, 0.0);
        assert_eq!(codec.detect_score("12345678901234567890").confidence, 0.7);
        assert_eq!(codec.detect_score("1234567890123456789,").confidence, 0.0);
    }

    #[test]
    fn test_ipv6_roundtrip_various() {
        let codec = Ipv6;
        let test_cases = vec![
            ("2001:db8::1", "2001:db8::1"),
            ("fe80::1", "fe80::1"),
            ("ff02::1", "ff02::1"),
            ("2001:0db8:85a3:0000:0000:8a2e:0370:7334", "2001:db8:85a3::8a2e:370:7334"),
            ("::ffff:192.0.2.1", "::ffff:192.0.2.1"),
            ("2001:DB8::1", "2001:db8::1"), // uppercase normalization
        ];

        for (input, expected_output) in test_cases {
            let encoded = codec.encode(input.as_bytes()).unwrap();
            assert_eq!(encoded.len(), 20, "encoding failed for {}", input);
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            let decoded_str = std::str::from_utf8(&decoded).unwrap();
            assert_eq!(decoded_str, expected_output, "roundtrip failed for {}", input);
        }
    }

    #[test]
    fn test_ipv6_compressed_forms() {
        let codec = Ipv6;

        // Different representations of same address should produce same encoding
        let addr1 = codec.encode(b"2001:0db8:0000:0000:0000:0000:0000:0001").unwrap();
        let addr2 = codec.encode(b"2001:db8::1").unwrap();
        assert_eq!(addr1, addr2);

        // Decoding should give canonical form
        let decoded = codec.decode(&addr1, Mode::Strict).unwrap();
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), "2001:db8::1");
    }

    #[test]
    fn test_ipv6_with_whitespace() {
        let codec = Ipv6;
        // Leading/trailing whitespace should be handled
        let encoded1 = codec.encode(b"  ::1  ").unwrap();
        let encoded2 = codec.encode(b"::1").unwrap();
        assert_eq!(encoded1, encoded2);
    }
}
