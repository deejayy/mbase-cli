use data_encoding::{Encoding, Specification};
use std::sync::OnceLock;

use super::Codec;
use super::util;
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const RFC4648_LOWER: &str = "abcdefghijklmnopqrstuvwxyz234567";
const RFC4648_UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const HEX_LOWER: &str = "0123456789abcdefghijklmnopqrstuv";
const HEX_UPPER: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUV";

fn make_encoding(alphabet: &str, padding: bool) -> Encoding {
    let mut spec = Specification::new();
    spec.symbols.push_str(alphabet);
    if padding {
        spec.padding = Some('=');
    }
    spec.encoding().unwrap()
}

static BASE32_LOWER: OnceLock<Encoding> = OnceLock::new();
static BASE32_UPPER: OnceLock<Encoding> = OnceLock::new();
static BASE32_PAD_LOWER: OnceLock<Encoding> = OnceLock::new();
static BASE32_PAD_UPPER: OnceLock<Encoding> = OnceLock::new();
static BASE32_HEX_LOWER: OnceLock<Encoding> = OnceLock::new();
static BASE32_HEX_UPPER: OnceLock<Encoding> = OnceLock::new();
static BASE32_HEX_PAD_LOWER: OnceLock<Encoding> = OnceLock::new();
static BASE32_HEX_PAD_UPPER: OnceLock<Encoding> = OnceLock::new();

fn get_base32_lower() -> &'static Encoding {
    BASE32_LOWER.get_or_init(|| make_encoding(RFC4648_LOWER, false))
}
fn get_base32_upper() -> &'static Encoding {
    BASE32_UPPER.get_or_init(|| make_encoding(RFC4648_UPPER, false))
}
fn get_base32_pad_lower() -> &'static Encoding {
    BASE32_PAD_LOWER.get_or_init(|| make_encoding(RFC4648_LOWER, true))
}
fn get_base32_pad_upper() -> &'static Encoding {
    BASE32_PAD_UPPER.get_or_init(|| make_encoding(RFC4648_UPPER, true))
}
fn get_base32_hex_lower() -> &'static Encoding {
    BASE32_HEX_LOWER.get_or_init(|| make_encoding(HEX_LOWER, false))
}
fn get_base32_hex_upper() -> &'static Encoding {
    BASE32_HEX_UPPER.get_or_init(|| make_encoding(HEX_UPPER, false))
}
fn get_base32_hex_pad_lower() -> &'static Encoding {
    BASE32_HEX_PAD_LOWER.get_or_init(|| make_encoding(HEX_LOWER, true))
}
fn get_base32_hex_pad_upper() -> &'static Encoding {
    BASE32_HEX_PAD_UPPER.get_or_init(|| make_encoding(HEX_UPPER, true))
}

fn decode_base32(
    input: &str,
    mode: Mode,
    enc: &Encoding,
    pad_enc: &Encoding,
    expects_padding: bool,
    is_lowercase: bool,
) -> Result<Vec<u8>> {
    let cleaned = util::clean_for_mode(input, mode);

    match mode {
        Mode::Strict => {
            let e = if expects_padding { pad_enc } else { enc };
            e.decode(cleaned.as_bytes())
                .map_err(|e| MbaseError::invalid_input(e.to_string()))
        }
        Mode::Lenient => {
            let normalized = if is_lowercase {
                cleaned.to_lowercase()
            } else {
                cleaned.to_uppercase()
            };
            let stripped = normalized.trim_end_matches('=').trim_end_matches('=');
            enc.decode(stripped.as_bytes())
                .or_else(|_| {
                    let padded = pad_to_base32(stripped);
                    pad_enc.decode(padded.as_bytes())
                })
                .map_err(|e| MbaseError::invalid_input(e.to_string()))
        }
    }
}

fn pad_to_base32(input: &str) -> String {
    let remainder = input.len() % 8;
    if remainder == 0 {
        return input.to_string();
    }
    let padding = match remainder {
        2 => 6,
        4 => 4,
        5 => 3,
        7 => 1,
        _ => 0,
    };
    format!("{}{}", input, "=".repeat(padding))
}

fn detect_base32(input: &str, codec_name: &str, alphabet: &str, multibase_code: char, expects_padding: bool) -> DetectCandidate {
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();
    let warnings = Vec::new();

    if input.is_empty() {
        return DetectCandidate {
            codec: codec_name.to_string(),
            confidence: 0.0,
            reasons: vec!["empty input".to_string()],
            warnings: vec![],
        };
    }

    if input.starts_with(multibase_code) {
        confidence = util::confidence::MULTIBASE_MATCH;
        reasons.push(format!("multibase prefix '{}' detected", multibase_code));
    }

    let valid_chars = input.chars().filter(|c| alphabet.contains(*c) || *c == '=').count();
    let ratio = valid_chars as f64 / input.len() as f64;

    if ratio == 1.0 {
        confidence = confidence.max(util::confidence::ALPHABET_MATCH);
        reasons.push("all characters valid".to_string());
    } else if ratio >= 0.9 {
        confidence = confidence.max(util::confidence::WEAK_MATCH);
    }

    let has_padding = input.contains('=');
    if expects_padding == has_padding {
        confidence += 0.1;
    }

    DetectCandidate {
        codec: codec_name.to_string(),
        confidence: confidence.min(1.0),
        reasons,
        warnings,
    }
}

macro_rules! impl_base32_codec {
    ($name:ident, $codec_name:expr, $aliases:expr, $alphabet:expr, $multibase:expr,
     $case:expr, $padding_rule:expr, $expects_padding:expr, $is_lowercase:expr, $desc:expr,
     $enc_fn:expr, $pad_enc_fn:expr) => {
        pub struct $name;

        impl Codec for $name {
            fn meta(&self) -> CodecMeta {
                CodecMeta {
                    name: $codec_name,
                    aliases: $aliases,
                    alphabet: $alphabet,
                    multibase_code: $multibase,
                    padding: $padding_rule,
                    case_sensitivity: $case,
                    description: $desc,
                }
            }

            fn encode(&self, input: &[u8]) -> Result<String> {
                let enc = if $expects_padding { $pad_enc_fn() } else { $enc_fn() };
                Ok(enc.encode(input))
            }

            fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
                decode_base32(input, mode, $enc_fn(), $pad_enc_fn(), $expects_padding, $is_lowercase)
            }

            fn detect_score(&self, input: &str) -> DetectCandidate {
                detect_base32(input, $codec_name, $alphabet, $multibase.unwrap_or(' '), $expects_padding)
            }
        }
    };
}

impl_base32_codec!(
    Base32Lower, "base32lower", &["base32", "b32"], RFC4648_LOWER, Some('b'),
    CaseSensitivity::Lower, PaddingRule::None, false, true,
    "RFC4648 Base32 lowercase without padding",
    get_base32_lower, get_base32_pad_lower
);

impl_base32_codec!(
    Base32Upper, "base32upper", &["B32"], RFC4648_UPPER, Some('B'),
    CaseSensitivity::Upper, PaddingRule::None, false, false,
    "RFC4648 Base32 uppercase without padding",
    get_base32_upper, get_base32_pad_upper
);

impl_base32_codec!(
    Base32PadLower, "base32padlower", &["base32pad", "b32pad"], RFC4648_LOWER, Some('c'),
    CaseSensitivity::Lower, PaddingRule::Required, true, true,
    "RFC4648 Base32 lowercase with padding",
    get_base32_lower, get_base32_pad_lower
);

impl_base32_codec!(
    Base32PadUpper, "base32padupper", &["B32PAD"], RFC4648_UPPER, Some('C'),
    CaseSensitivity::Upper, PaddingRule::Required, true, false,
    "RFC4648 Base32 uppercase with padding",
    get_base32_upper, get_base32_pad_upper
);

impl_base32_codec!(
    Base32HexLower, "base32hexlower", &["base32hex", "b32hex"], HEX_LOWER, Some('v'),
    CaseSensitivity::Lower, PaddingRule::None, false, true,
    "RFC4648 Base32hex lowercase without padding",
    get_base32_hex_lower, get_base32_hex_pad_lower
);

impl_base32_codec!(
    Base32HexUpper, "base32hexupper", &["B32HEX"], HEX_UPPER, Some('V'),
    CaseSensitivity::Upper, PaddingRule::None, false, false,
    "RFC4648 Base32hex uppercase without padding",
    get_base32_hex_upper, get_base32_hex_pad_upper
);

impl_base32_codec!(
    Base32HexPadLower, "base32hexpadlower", &["base32hexpad", "b32hexpad"], HEX_LOWER, Some('t'),
    CaseSensitivity::Lower, PaddingRule::Required, true, true,
    "RFC4648 Base32hex lowercase with padding",
    get_base32_hex_lower, get_base32_hex_pad_lower
);

impl_base32_codec!(
    Base32HexPadUpper, "base32hexpadupper", &["B32HEXPAD"], HEX_UPPER, Some('T'),
    CaseSensitivity::Upper, PaddingRule::Required, true, false,
    "RFC4648 Base32hex uppercase with padding",
    get_base32_hex_upper, get_base32_hex_pad_upper
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base32_encode() {
        assert_eq!(Base32Lower.encode(b"Hello").unwrap(), "jbswy3dp");
        assert_eq!(Base32Upper.encode(b"Hello").unwrap(), "JBSWY3DP");
    }

    #[test]
    fn test_base32_decode() {
        assert_eq!(Base32Lower.decode("jbswy3dp", Mode::Strict).unwrap(), b"Hello");
        assert_eq!(Base32Upper.decode("JBSWY3DP", Mode::Strict).unwrap(), b"Hello");
    }

    #[test]
    fn test_base32_pad_encode() {
        assert_eq!(Base32PadLower.encode(b"Hello").unwrap(), "jbswy3dp");
        assert_eq!(Base32PadLower.encode(b"Hi").unwrap(), "jbuq====");
    }

    #[test]
    fn test_base32_pad_decode() {
        assert_eq!(Base32PadLower.decode("jbuq====", Mode::Strict).unwrap(), b"Hi");
    }

    #[test]
    fn test_base32_roundtrip() {
        let data = b"The quick brown fox";
        for codec in [&Base32Lower as &dyn Codec, &Base32Upper, &Base32PadLower, &Base32PadUpper] {
            let encoded = codec.encode(data).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, data, "roundtrip failed for {}", codec.name());
        }
    }

    #[test]
    fn test_base32hex_encode() {
        assert_eq!(Base32HexLower.encode(b"Hello").unwrap(), "91imor3f");
        assert_eq!(Base32HexUpper.encode(b"Hello").unwrap(), "91IMOR3F");
    }

    #[test]
    fn test_base32_lenient_case() {
        assert_eq!(Base32Lower.decode("JBSWY3DP", Mode::Lenient).unwrap(), b"Hello");
    }

    #[test]
    fn test_base32_lenient_padding() {
        assert_eq!(Base32PadLower.decode("jbuq", Mode::Lenient).unwrap(), b"Hi");
    }

    #[test]
    fn test_base32_strict_rejects_padding() {
        assert!(Base32Lower.validate("jbswy3dp======", Mode::Strict).is_err());
    }

    #[test]
    fn test_base32_empty() {
        assert_eq!(Base32Lower.encode(&[]).unwrap(), "");
        assert_eq!(Base32Lower.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }
}
