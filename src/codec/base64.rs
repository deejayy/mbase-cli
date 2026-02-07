use base64::prelude::*;
use base64::Engine;

use super::Codec;
use super::util;
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const STANDARD_ALPHABET: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn validate_padding(input: &str, padding_rule: PaddingRule) -> Result<()> {
    let pad_count = input.chars().rev().take_while(|&c| c == '=').count();
    let has_padding = pad_count > 0;

    match padding_rule {
        PaddingRule::Required if !has_padding && input.len() % 4 != 0 => {
            Err(MbaseError::invalid_padding("padding required"))
        }
        PaddingRule::None if has_padding => {
            Err(MbaseError::invalid_padding("padding not allowed"))
        }
        _ => {
            if pad_count > 2 {
                return Err(MbaseError::invalid_padding(
                    "too many padding characters",
                ));
            }
            Ok(())
        }
    }
}

fn detect_base64_common(
    input: &str,
    codec_name: &str,
    alphabet: &str,
    multibase_code: char,
    expects_padding: bool,
) -> DetectCandidate {
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();
    let mut warnings = Vec::new();

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

    let valid_chars = input
        .chars()
        .filter(|c| alphabet.contains(*c) || *c == '=')
        .count();
    let total_chars = input.len();
    let char_ratio = valid_chars as f64 / total_chars as f64;

    if char_ratio == 1.0 {
        confidence = confidence.max(util::confidence::ALPHABET_MATCH);
        reasons.push("all characters valid".to_string());
    } else if char_ratio >= 0.9 {
        confidence = confidence.max(0.4);
        warnings.push(format!(
            "{:.1}% invalid characters",
            (1.0 - char_ratio) * 100.0
        ));
    } else {
        confidence = 0.0;
        warnings.push("too many invalid characters".to_string());
    }

    let has_padding = input.contains('=');
    if expects_padding && has_padding {
        confidence += 0.1;
        reasons.push("has expected padding".to_string());
    } else if !expects_padding && !has_padding {
        confidence += 0.05;
        reasons.push("no padding as expected".to_string());
    } else if expects_padding && !has_padding {
        warnings.push("expected padding not found".to_string());
    }

    let len_mod = input.trim_end_matches('=').len() % 4;
    if len_mod == 1 {
        confidence *= 0.5;
        warnings.push("invalid length (mod 4 = 1)".to_string());
    }

    DetectCandidate {
        codec: codec_name.to_string(),
        confidence: confidence.min(1.0),
        reasons,
        warnings,
    }
}

pub struct Base64;

impl Codec for Base64 {
    fn name(&self) -> &'static str {
        "base64"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base64",
            aliases: &["b64", "std64"],
            alphabet: STANDARD_ALPHABET,
            multibase_code: Some('m'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "RFC4648 Base64 without padding",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(BASE64_STANDARD_NO_PAD.encode(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);
        let to_decode = match mode {
            Mode::Strict => {
                self.validate(&cleaned, mode)?;
                cleaned
            }
            Mode::Lenient => cleaned.trim_end_matches('=').to_string(),
        };
        BASE64_STANDARD_NO_PAD
            .decode(&to_decode)
            .map_err(|e| MbaseError::invalid_input(e.to_string()))
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        let cleaned = util::clean_for_mode(input, mode);
        util::validate_alphabet_with_padding(&cleaned, STANDARD_ALPHABET, false)?;
        validate_padding(&cleaned, PaddingRule::None)?;
        Ok(())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_base64_common(input, "base64", STANDARD_ALPHABET, 'm', false)
    }
}

pub struct Base64Pad;

impl Codec for Base64Pad {
    fn name(&self) -> &'static str {
        "base64pad"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base64pad",
            aliases: &["b64pad"],
            alphabet: STANDARD_ALPHABET,
            multibase_code: Some('M'),
            padding: PaddingRule::Required,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "RFC4648 Base64 with required padding",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(BASE64_STANDARD.encode(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);
        match mode {
            Mode::Strict => {
                self.validate(&cleaned, mode)?;
                BASE64_STANDARD
                    .decode(&cleaned)
                    .map_err(|e| MbaseError::invalid_input(e.to_string()))
            }
            Mode::Lenient => {
                let padded = pad_to_multiple(&cleaned, 4);
                BASE64_STANDARD
                    .decode(&padded)
                    .map_err(|e| MbaseError::invalid_input(e.to_string()))
            }
        }
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        let cleaned = util::clean_for_mode(input, mode);
        util::validate_alphabet_with_padding(&cleaned, STANDARD_ALPHABET, true)?;
        validate_padding(&cleaned, PaddingRule::Required)?;
        Ok(())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_base64_common(input, "base64pad", STANDARD_ALPHABET, 'M', true)
    }
}

pub struct Base64Url;

impl Codec for Base64Url {
    fn name(&self) -> &'static str {
        "base64url"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base64url",
            aliases: &["b64url", "url64"],
            alphabet: URL_ALPHABET,
            multibase_code: Some('u'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "RFC4648 Base64url without padding",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(BASE64_URL_SAFE_NO_PAD.encode(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);
        let to_decode = match mode {
            Mode::Strict => {
                self.validate(&cleaned, mode)?;
                cleaned
            }
            Mode::Lenient => cleaned.trim_end_matches('=').to_string(),
        };
        BASE64_URL_SAFE_NO_PAD
            .decode(&to_decode)
            .map_err(|e| MbaseError::invalid_input(e.to_string()))
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        let cleaned = util::clean_for_mode(input, mode);
        util::validate_alphabet_with_padding(&cleaned, URL_ALPHABET, false)?;
        validate_padding(&cleaned, PaddingRule::None)?;
        Ok(())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_base64_common(input, "base64url", URL_ALPHABET, 'u', false)
    }
}

pub struct Base64UrlPad;

impl Codec for Base64UrlPad {
    fn name(&self) -> &'static str {
        "base64urlpad"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base64urlpad",
            aliases: &["b64urlpad"],
            alphabet: URL_ALPHABET,
            multibase_code: Some('U'),
            padding: PaddingRule::Required,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "RFC4648 Base64url with required padding",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(BASE64_URL_SAFE.encode(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);
        match mode {
            Mode::Strict => {
                self.validate(&cleaned, mode)?;
                BASE64_URL_SAFE
                    .decode(&cleaned)
                    .map_err(|e| MbaseError::invalid_input(e.to_string()))
            }
            Mode::Lenient => {
                let padded = pad_to_multiple(&cleaned, 4);
                BASE64_URL_SAFE
                    .decode(&padded)
                    .map_err(|e| MbaseError::invalid_input(e.to_string()))
            }
        }
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        let cleaned = util::clean_for_mode(input, mode);
        util::validate_alphabet_with_padding(&cleaned, URL_ALPHABET, true)?;
        validate_padding(&cleaned, PaddingRule::Required)?;
        Ok(())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_base64_common(input, "base64urlpad", URL_ALPHABET, 'U', true)
    }
}

fn pad_to_multiple(input: &str, multiple: usize) -> String {
    let stripped = input.trim_end_matches('=');
    let remainder = stripped.len() % multiple;
    if remainder == 0 {
        stripped.to_string()
    } else {
        let padding_needed = multiple - remainder;
        format!("{}{}", stripped, "=".repeat(padding_needed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode_empty() {
        assert_eq!(Base64.encode(&[]).unwrap(), "");
    }

    #[test]
    fn test_base64_encode_hello() {
        assert_eq!(Base64.encode(b"Hello").unwrap(), "SGVsbG8");
    }

    #[test]
    fn test_base64_decode_hello() {
        assert_eq!(
            Base64.decode("SGVsbG8", Mode::Strict).unwrap(),
            b"Hello".to_vec()
        );
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = b"The quick brown fox jumps over the lazy dog";
        let encoded = Base64.encode(data).unwrap();
        let decoded = Base64.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data.to_vec());
    }

    #[test]
    fn test_base64pad_encode() {
        assert_eq!(Base64Pad.encode(b"Hello").unwrap(), "SGVsbG8=");
        assert_eq!(Base64Pad.encode(b"He").unwrap(), "SGU=");
        assert_eq!(Base64Pad.encode(b"Hel").unwrap(), "SGVs");
    }

    #[test]
    fn test_base64pad_decode() {
        assert_eq!(
            Base64Pad.decode("SGVsbG8=", Mode::Strict).unwrap(),
            b"Hello".to_vec()
        );
    }

    #[test]
    fn test_base64url_encode() {
        let data = b"\xfb\xff\xfe";
        let std = Base64.encode(data).unwrap();
        let url = Base64Url.encode(data).unwrap();
        assert!(std.contains('+') || std.contains('/'));
        assert!(!url.contains('+') && !url.contains('/'));
    }

    #[test]
    fn test_lenient_whitespace() {
        let input = "SGVs\nbG8=";
        assert!(Base64Pad.decode(input, Mode::Strict).is_err());
        assert_eq!(
            Base64Pad.decode(input, Mode::Lenient).unwrap(),
            b"Hello".to_vec()
        );
    }

    #[test]
    fn test_lenient_missing_padding() {
        assert_eq!(
            Base64Pad.decode("SGVsbG8", Mode::Lenient).unwrap(),
            b"Hello".to_vec()
        );
    }

    #[test]
    fn test_strict_rejects_padding_on_nopad() {
        assert!(Base64.validate("SGVsbG8=", Mode::Strict).is_err());
    }

    #[test]
    fn test_invalid_character() {
        let result = Base64.validate("SGVs!G8", Mode::Strict);
        assert!(matches!(result, Err(MbaseError::InvalidCharacter { char: '!', position: 4 })));
    }

    #[test]
    fn test_detect_multibase_prefix() {
        let candidate = Base64.detect_score("mSGVsbG8");
        assert!(candidate.confidence > 0.9);
        assert!(candidate.reasons.iter().any(|r| r.contains("multibase")));
    }
}
