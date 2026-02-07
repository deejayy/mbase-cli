use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const BLOCK_START: [u32; 256] = [
    0x03400, 0x03500, 0x03600, 0x03700, 0x03800, 0x03900, 0x03A00, 0x03B00, 0x03C00, 0x03D00, 0x03E00, 0x03F00, 0x04000, 0x04100, 0x04200,
    0x04300, 0x04400, 0x04500, 0x04600, 0x04700, 0x04800, 0x04900, 0x04A00, 0x04B00, 0x04C00, 0x04E00, 0x04F00, 0x05000, 0x05100, 0x05200,
    0x05300, 0x05400, 0x05500, 0x05600, 0x05700, 0x05800, 0x05900, 0x05A00, 0x05B00, 0x05C00, 0x05D00, 0x05E00, 0x05F00, 0x06000, 0x06100,
    0x06200, 0x06300, 0x06400, 0x06500, 0x06600, 0x06700, 0x06800, 0x06900, 0x06A00, 0x06B00, 0x06C00, 0x06D00, 0x06E00, 0x06F00, 0x07000,
    0x07100, 0x07200, 0x07300, 0x07400, 0x07500, 0x07600, 0x07700, 0x07800, 0x07900, 0x07A00, 0x07B00, 0x07C00, 0x07D00, 0x07E00, 0x07F00,
    0x08000, 0x08100, 0x08200, 0x08300, 0x08400, 0x08500, 0x08600, 0x08700, 0x08800, 0x08900, 0x08A00, 0x08B00, 0x08C00, 0x08D00, 0x08E00,
    0x08F00, 0x09000, 0x09100, 0x09200, 0x09300, 0x09400, 0x09500, 0x09600, 0x09700, 0x09800, 0x09900, 0x09A00, 0x09B00, 0x09C00, 0x09D00,
    0x09E00, 0x09F00, 0x0A000, 0x0A100, 0x0A200, 0x0A300, 0x0A400, 0x0A500, 0x0A600, 0x0A700, 0x0A800, 0x0A900, 0x0AA00, 0x0AB00, 0x0AC00,
    0x0AD00, 0x0AE00, 0x0AF00, 0x0B000, 0x0B100, 0x0B200, 0x0B300, 0x0B400, 0x0B500, 0x0B600, 0x0B700, 0x0B800, 0x0B900, 0x0BA00, 0x0BB00,
    0x0BC00, 0x0BD00, 0x0BE00, 0x0BF00, 0x0C000, 0x0C100, 0x0C200, 0x0C300, 0x0C400, 0x0C500, 0x0C600, 0x0C700, 0x0C800, 0x0C900, 0x0CA00,
    0x0CB00, 0x0CC00, 0x0CD00, 0x0CE00, 0x0CF00, 0x0D000, 0x0D100, 0x0D200, 0x0D300, 0x0D400, 0x0D500, 0x0D600, 0x0D700, 0x10000, 0x10100,
    0x10200, 0x10300, 0x10400, 0x10500, 0x10600, 0x10700, 0x10800, 0x10900, 0x10A00, 0x10B00, 0x10C00, 0x10D00, 0x10E00, 0x10F00, 0x11000,
    0x11100, 0x11200, 0x11300, 0x11400, 0x11500, 0x11600, 0x11700, 0x11800, 0x11900, 0x11A00, 0x11B00, 0x11C00, 0x11D00, 0x11E00, 0x11F00,
    0x12000, 0x12100, 0x12200, 0x12300, 0x12400, 0x12500, 0x13000, 0x13100, 0x13200, 0x13300, 0x13400, 0x14400, 0x14500, 0x14600, 0x16800,
    0x16900, 0x16A00, 0x16B00, 0x16F00, 0x17000, 0x18700, 0x18800, 0x18900, 0x18A00, 0x18B00, 0x18C00, 0x18D00, 0x1B000, 0x1B100, 0x1B200,
    0x1B300, 0x1BC00, 0x1D000, 0x1D100, 0x1D200, 0x1D300, 0x1D400, 0x1D500, 0x1D600, 0x1D700, 0x1E800, 0x1E900, 0x1EC00, 0x1ED00, 0x1EE00,
    0x1F000, 0x1F100, 0x1F200, 0x1F300, 0x1F400, 0x1F500, 0x1F600, 0x1F700, 0x1F800, 0x1F900, 0x1FA00, 0x1FB00, 0x20000, 0x2A700, 0x2B700,
    0x2B800,
];

const PADDING_BLOCK_START: u32 = 0x01800;

fn build_reverse_map() -> std::collections::HashMap<u32, (u8, u8)> {
    let mut map = std::collections::HashMap::new();
    for (hi, &base) in BLOCK_START.iter().enumerate() {
        for lo in 0u32..256 {
            map.insert(base + lo, (hi as u8, lo as u8));
        }
    }
    for lo in 0u32..256 {
        map.insert(PADDING_BLOCK_START + lo, (255, lo as u8));
    }
    map
}

pub struct Base65536;

impl Codec for Base65536 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base65536",
            aliases: &["b65536"],
            alphabet: "Unicode BMP safe blocks",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Base65536 encoding (2 bytes per Unicode char)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        if input.is_empty() {
            return Ok(String::new());
        }

        let mut result = String::new();
        let mut iter = input.iter().peekable();

        while let Some(&hi) = iter.next() {
            if let Some(&lo) = iter.next() {
                let codepoint = BLOCK_START[hi as usize] + (lo as u32);
                result.push(char::from_u32(codepoint).unwrap());
            } else {
                let codepoint = PADDING_BLOCK_START + (hi as u32);
                result.push(char::from_u32(codepoint).unwrap());
            }
        }

        Ok(result)
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let input = if mode == Mode::Lenient {
            input.chars().filter(|c| !c.is_whitespace()).collect::<String>()
        } else {
            input.to_string()
        };

        if input.is_empty() {
            return Ok(Vec::new());
        }

        let reverse = build_reverse_map();
        let mut result = Vec::new();
        let chars: Vec<char> = input.chars().collect();

        for (pos, &c) in chars.iter().enumerate() {
            let cp = c as u32;
            if let Some(&(hi, lo)) = reverse.get(&cp) {
                if hi == 255 {
                    if pos != chars.len() - 1 {
                        return Err(MbaseError::invalid_input("padding character in non-final position"));
                    }
                    result.push(lo);
                } else {
                    result.push(hi);
                    result.push(lo);
                }
            } else {
                return Err(MbaseError::InvalidCharacter { char: c, position: pos });
            }
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

        let reverse = build_reverse_map();
        let total = clean.chars().count();
        let valid = clean.chars().filter(|c| reverse.contains_key(&(*c as u32))).count();

        if valid == 0 {
            return DetectCandidate {
                codec: self.name().to_string(),
                confidence: 0.0,
                reasons: vec!["no valid base65536 characters".to_string()],
                warnings: vec![],
            };
        }

        let ratio = valid as f64 / total as f64;
        let all_high_unicode = clean.chars().all(|c| c as u32 > 0x3000);

        let confidence = if ratio > 0.9 && all_high_unicode {
            0.85
        } else if ratio > 0.8 {
            util::confidence::ALPHABET_MATCH
        } else {
            ratio * util::confidence::PARTIAL_MATCH
        };

        DetectCandidate {
            codec: self.name().to_string(),
            confidence,
            reasons: vec![
                format!("{}/{} valid base65536 characters", valid, total),
                if all_high_unicode {
                    "all high Unicode".to_string()
                } else {
                    "mixed".to_string()
                },
            ],
            warnings: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base65536_empty() {
        let codec = Base65536;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_base65536_single_byte() {
        let codec = Base65536;
        let encoded = codec.encode(b"A").unwrap();
        assert_eq!(encoded.chars().count(), 1);
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"A");
    }

    #[test]
    fn test_base65536_two_bytes() {
        let codec = Base65536;
        let encoded = codec.encode(b"AB").unwrap();
        assert_eq!(encoded.chars().count(), 1);
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"AB");
    }

    #[test]
    fn test_base65536_hello() {
        let codec = Base65536;
        let encoded = codec.encode(b"Hello").unwrap();
        assert_eq!(encoded.chars().count(), 3);
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn test_base65536_roundtrip() {
        let codec = Base65536;
        let inputs = [
            b"".to_vec(),
            b"a".to_vec(),
            b"ab".to_vec(),
            b"abc".to_vec(),
            b"Hello, World!".to_vec(),
            (0..=255).collect::<Vec<u8>>(),
        ];
        for input in inputs {
            let encoded = codec.encode(&input).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, input, "roundtrip failed for len {}", input.len());
        }
    }

    #[test]
    fn test_base65536_density() {
        let codec = Base65536;
        let input: Vec<u8> = (0..100).collect();
        let encoded = codec.encode(&input).unwrap();
        assert_eq!(encoded.chars().count(), 50);
    }

    #[test]
    fn test_base65536_lenient_whitespace() {
        let codec = Base65536;
        let encoded = codec.encode(b"test").unwrap();
        let with_space = format!("{} {}", encoded.chars().take(1).collect::<String>(), encoded.chars().skip(1).collect::<String>());
        let decoded = codec.decode(&with_space, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"test");
    }
}
