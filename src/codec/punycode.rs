use super::{util, Codec};
use crate::error::{MbaseError as Error, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct Punycode;

const BASE: u32 = 36;
const TMIN: u32 = 1;
const TMAX: u32 = 26;
const SKEW: u32 = 38;
const DAMP: u32 = 700;
const INITIAL_BIAS: u32 = 72;
const INITIAL_N: u32 = 0x80;

fn adapt(mut delta: u32, numpoints: u32, firsttime: bool) -> u32 {
    delta = if firsttime { delta / DAMP } else { delta / 2 };
    delta += delta / numpoints;

    let mut k = 0;
    while delta > ((BASE - TMIN) * TMAX) / 2 {
        delta /= BASE - TMIN;
        k += BASE;
    }

    k + (((BASE - TMIN + 1) * delta) / (delta + SKEW))
}

fn encode_digit(d: u32) -> char {
    if d < 26 {
        (b'a' + d as u8) as char
    } else {
        (b'0' + (d - 26) as u8) as char
    }
}

fn decode_digit(c: char) -> Option<u32> {
    match c {
        'a'..='z' => Some((c as u8 - b'a') as u32),
        'A'..='Z' => Some((c as u8 - b'A') as u32),
        '0'..='9' => Some(26 + (c as u8 - b'0') as u32),
        _ => None,
    }
}

impl Codec for Punycode {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "punycode",
            aliases: &["pcode"],
            alphabet: "abcdefghijklmnopqrstuvwxyz0123456789-",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "Punycode (RFC3492 IDN encoding)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let input_str = std::str::from_utf8(input).map_err(|e| Error::invalid_input(format!("invalid UTF-8: {}", e)))?;

        let chars: Vec<u32> = input_str.chars().map(|c| c as u32).collect();

        let mut output = String::new();
        let basic: Vec<char> = chars.iter().filter(|&&c| c < 0x80).map(|&c| c as u8 as char).collect();

        let h = basic.len();
        let b = h;

        output.push_str(&basic.iter().collect::<String>());
        if b > 0 && b < chars.len() {
            output.push('-');
        }

        let mut n = INITIAL_N;
        let mut delta = 0u32;
        let mut bias = INITIAL_BIAS;
        let mut h = h;

        while h < chars.len() {
            let m = *chars.iter().filter(|&&c| c >= n).min().unwrap();

            delta = delta
                .checked_add(
                    (m - n)
                        .checked_mul((h + 1) as u32)
                        .ok_or_else(|| Error::invalid_input("overflow"))?,
                )
                .ok_or_else(|| Error::invalid_input("overflow"))?;
            n = m;

            for &c in &chars {
                if c < n {
                    delta = delta.checked_add(1).ok_or_else(|| Error::invalid_input("overflow"))?;
                } else if c == n {
                    let mut q = delta;
                    let mut k = BASE;

                    loop {
                        let t = if k <= bias {
                            TMIN
                        } else if k >= bias + TMAX {
                            TMAX
                        } else {
                            k - bias
                        };

                        if q < t {
                            break;
                        }

                        output.push(encode_digit(t + ((q - t) % (BASE - t))));
                        q = (q - t) / (BASE - t);
                        k += BASE;
                    }

                    output.push(encode_digit(q));
                    bias = adapt(delta, (h + 1) as u32, h == b);
                    delta = 0;
                    h += 1;
                }
            }

            delta += 1;
            n += 1;
        }

        Ok(output)
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

        let input_lower = cleaned.to_lowercase();
        let delimiter_pos = input_lower.rfind('-');

        let (basic, encoded) = match delimiter_pos {
            Some(pos) => {
                let basic = &input_lower[..pos];
                let encoded = &input_lower[pos + 1..];
                (basic.to_string(), encoded)
            }
            None => {
                // No delimiter means all characters are basic (ASCII-only)
                (input_lower.clone(), "")
            }
        };

        let mut output: Vec<u32> = basic.chars().map(|c| c as u32).collect();
        let mut n = INITIAL_N;
        let mut i = 0u32;
        let mut bias = INITIAL_BIAS;

        let mut pos = 0;
        while pos < encoded.len() {
            let oldi = i;
            let mut w = 1u32;
            let mut k = BASE;

            loop {
                if pos >= encoded.len() {
                    return Err(Error::invalid_input("truncated input"));
                }

                let c = encoded.chars().nth(pos).unwrap();
                pos += 1;

                let digit = decode_digit(c).ok_or_else(|| Error::invalid_input(format!("invalid punycode digit: '{}'", c)))?;

                i = i
                    .checked_add(digit.checked_mul(w).ok_or_else(|| Error::invalid_input("overflow"))?)
                    .ok_or_else(|| Error::invalid_input("overflow"))?;

                let t = if k <= bias {
                    TMIN
                } else if k >= bias + TMAX {
                    TMAX
                } else {
                    k - bias
                };

                if digit < t {
                    break;
                }

                w = w.checked_mul(BASE - t).ok_or_else(|| Error::invalid_input("overflow"))?;
                k += BASE;
            }

            bias = adapt(i - oldi, (output.len() + 1) as u32, oldi == 0);
            n = n
                .checked_add(i / ((output.len() + 1) as u32))
                .ok_or_else(|| Error::invalid_input("overflow"))?;
            i %= (output.len() + 1) as u32;

            output.insert(i as usize, n);
            i += 1;
        }

        let result: String = output.iter().filter_map(|&c| char::from_u32(c)).collect();

        Ok(result.into_bytes())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        if input.is_empty() {
            return DetectCandidate {
                codec: "punycode".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            };
        }

        let lower = input.to_lowercase();
        let valid_chars = lower
            .chars()
            .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .count();

        let ratio = valid_chars as f32 / input.len() as f32;
        let has_delimiter = lower.contains('-');
        let has_digit = lower.chars().any(|c| c.is_ascii_digit());

        if ratio < 0.95 {
            // Too many invalid characters
            return DetectCandidate {
                codec: "punycode".to_string(),
                confidence: 0.0,
                reasons: vec![],
                warnings: vec![],
            };
        }

        if ratio > 0.95 && has_delimiter {
            // Punycode has delimiter and all valid chars - strong match
            DetectCandidate {
                codec: "punycode".to_string(),
                confidence: util::confidence::PARTIAL_MATCH,
                reasons: vec!["valid punycode pattern with delimiter".to_string()],
                warnings: vec![],
            }
        } else if ratio > 0.95 && has_digit {
            DetectCandidate {
                codec: "punycode".to_string(),
                confidence: util::confidence::WEAK_MATCH,
                reasons: vec!["valid chars with digit".to_string()],
                warnings: vec![],
            }
        } else if ratio > 0.95 {
            // Plain ASCII, could be punycode
            DetectCandidate {
                codec: "punycode".to_string(),
                confidence: 0.25,
                reasons: vec!["all valid ASCII chars".to_string()],
                warnings: vec![],
            }
        } else if ratio > 0.8 {
            DetectCandidate {
                codec: "punycode".to_string(),
                confidence: 0.2,
                reasons: vec!["mostly valid chars".to_string()],
                warnings: vec![],
            }
        } else {
            DetectCandidate {
                codec: "punycode".to_string(),
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
    fn test_punycode_encode_basic() {
        let codec = Punycode;
        let encoded = codec.encode(b"hello").unwrap();
        assert_eq!(encoded, "hello");
    }

    #[test]
    fn test_punycode_decode_basic() {
        let codec = Punycode;
        assert_eq!(codec.decode("hello", Mode::Strict).unwrap(), b"hello");
    }

    #[test]
    fn test_punycode_roundtrip_ascii() {
        let codec = Punycode;
        let test_cases = vec![b"test" as &[u8], b"hello", b"world"];

        for original in test_cases {
            let encoded = codec.encode(original).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, original, "roundtrip failed for {:?}", original);
        }
    }

    #[test]
    fn test_punycode_known_vectors() {
        let codec = Punycode;

        assert_eq!(codec.encode("bücher".as_bytes()).unwrap(), "bcher-kva");
        assert_eq!(codec.decode("bcher-kva", Mode::Strict).unwrap(), "bücher".as_bytes());

        assert_eq!(codec.decode("maana-pta", Mode::Strict).unwrap(), "mañana".as_bytes());
    }

    #[test]
    fn test_punycode_empty() {
        let codec = Punycode;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_punycode_case_insensitive() {
        let codec = Punycode;
        let lower = codec.decode("bcher-kva", Mode::Strict).unwrap();
        let upper = codec.decode("BCHER-KVA", Mode::Strict).unwrap();
        assert_eq!(lower, upper);
    }

    #[test]
    fn test_punycode_lenient_whitespace() {
        let codec = Punycode;
        let with_space = "bcher -kva";
        let without_space = "bcher-kva";
        let decoded_with = codec.decode(with_space, Mode::Lenient).unwrap();
        let decoded_without = codec.decode(without_space, Mode::Strict).unwrap();
        assert_eq!(decoded_with, decoded_without);
    }

    #[test]
    fn test_punycode_detect() {
        let codec = Punycode;
        assert!(codec.detect_score("bcher-kva").confidence > 0.4);
        assert!(codec.detect_score("hello").confidence > 0.2);
        assert!(codec.detect_score("hello$world").confidence < 0.1);
    }

    #[test]
    fn test_punycode_invalid_utf8() {
        let codec = Punycode;
        let invalid = vec![0xFF, 0xFE, 0xFD];
        assert!(codec.encode(&invalid).is_err());
    }
}
