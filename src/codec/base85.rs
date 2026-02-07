use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const ASCII85_ALPHABET: &str = "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstu";

const Z85_ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#";

fn encode_ascii85(input: &[u8]) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut result = String::new();

    for chunk in input.chunks(4) {
        let mut val: u32 = 0;
        for (i, &byte) in chunk.iter().enumerate() {
            val |= (byte as u32) << (24 - i * 8);
        }

        if chunk.len() == 4 && val == 0 {
            result.push('z');
        } else {
            let output_len = chunk.len() + 1;
            let mut chars = [0u8; 5];
            let mut v = val;
            for i in (0..5).rev() {
                chars[i] = (v % 85) as u8;
                v /= 85;
            }
            for item in chars.iter().take(output_len) {
                result.push((item + 33) as char);
            }
        }
    }

    result
}

fn decode_ascii85(input: &str, mode: Mode) -> Result<Vec<u8>> {
    let cleaned = util::clean_for_mode(input, mode);

    let stripped = if cleaned.starts_with("<~") && cleaned.ends_with("~>") {
        &cleaned[2..cleaned.len() - 2]
    } else {
        &cleaned
    };

    if stripped.is_empty() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    let mut chars: Vec<u8> = Vec::new();
    let mut pos = 0;

    for c in stripped.chars() {
        if c == 'z' {
            if !chars.is_empty() {
                return Err(MbaseError::invalid_input("'z' in middle of group"));
            }
            result.extend_from_slice(&[0, 0, 0, 0]);
            pos += 1;
            continue;
        }

        if !('!'..='u').contains(&c) {
            return Err(MbaseError::InvalidCharacter { char: c, position: pos });
        }

        chars.push(c as u8 - 33);
        pos += 1;

        if chars.len() == 5 {
            let val = chars.iter().fold(0u32, |acc, &v| acc * 85 + v as u32);
            result.push((val >> 24) as u8);
            result.push((val >> 16) as u8);
            result.push((val >> 8) as u8);
            result.push(val as u8);
            chars.clear();
        }
    }

    if !chars.is_empty() {
        let pad_count = 5 - chars.len();
        chars.extend(std::iter::repeat_n(84, pad_count));
        let val = chars.iter().fold(0u32, |acc, &v| acc * 85 + v as u32);
        let bytes = [(val >> 24) as u8, (val >> 16) as u8, (val >> 8) as u8, val as u8];
        result.extend_from_slice(&bytes[..4 - pad_count]);
    }

    Ok(result)
}

fn encode_z85(input: &[u8]) -> Result<String> {
    if !input.len().is_multiple_of(4) {
        return Err(MbaseError::invalid_length(crate::error::LengthConstraint::MultipleOf(4), input.len()));
    }

    let alphabet = Z85_ALPHABET.as_bytes();
    let mut result = String::new();

    for chunk in input.chunks(4) {
        let val = ((chunk[0] as u32) << 24) | ((chunk[1] as u32) << 16) | ((chunk[2] as u32) << 8) | (chunk[3] as u32);

        let mut chars = [0u8; 5];
        let mut v = val;
        for i in (0..5).rev() {
            chars[i] = alphabet[(v % 85) as usize];
            v /= 85;
        }
        for &c in &chars {
            result.push(c as char);
        }
    }

    Ok(result)
}

fn decode_z85(input: &str, mode: Mode) -> Result<Vec<u8>> {
    let cleaned = util::clean_for_mode(input, mode);

    if cleaned.is_empty() {
        return Ok(Vec::new());
    }

    if !cleaned.len().is_multiple_of(5) {
        return Err(MbaseError::invalid_length(crate::error::LengthConstraint::MultipleOf(5), cleaned.len()));
    }

    let mut result = Vec::new();

    for (chunk_idx, chunk) in cleaned.as_bytes().chunks(5).enumerate() {
        let mut val: u32 = 0;
        for (i, &c) in chunk.iter().enumerate() {
            let pos = chunk_idx * 5 + i;
            let v = Z85_ALPHABET
                .chars()
                .position(|x| x as u8 == c)
                .ok_or(MbaseError::InvalidCharacter {
                    char: c as char,
                    position: pos,
                })?;
            val = val * 85 + v as u32;
        }
        result.push((val >> 24) as u8);
        result.push((val >> 16) as u8);
        result.push((val >> 8) as u8);
        result.push(val as u8);
    }

    Ok(result)
}

fn detect_ascii85(input: &str) -> DetectCandidate {
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();

    if input.is_empty() {
        return DetectCandidate {
            codec: "ascii85".to_string(),
            confidence: 0.0,
            reasons: vec!["empty input".to_string()],
            warnings: vec![],
        };
    }

    if input.starts_with("<~") && input.ends_with("~>") {
        confidence = util::confidence::MULTIBASE_MATCH;
        reasons.push("has <~ ~> wrapper".to_string());
    }

    let valid = input.chars().filter(|&c| ('!'..='u').contains(&c) || c == 'z').count();
    let ratio = valid as f64 / input.len() as f64;

    if ratio > 0.9 {
        confidence = confidence.max(util::confidence::PARTIAL_MATCH);
        reasons.push(format!("{:.0}% valid ascii85 chars", ratio * 100.0));
    }

    DetectCandidate {
        codec: "ascii85".to_string(),
        confidence: confidence.min(1.0),
        reasons,
        warnings: vec![],
    }
}

fn detect_z85(input: &str) -> DetectCandidate {
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();

    if input.is_empty() {
        return DetectCandidate {
            codec: "z85".to_string(),
            confidence: 0.0,
            reasons: vec!["empty input".to_string()],
            warnings: vec![],
        };
    }

    let valid = input.chars().filter(|c| Z85_ALPHABET.contains(*c)).count();
    let ratio = valid as f64 / input.len() as f64;

    if ratio == 1.0 && input.len().is_multiple_of(5) {
        confidence = util::confidence::PARTIAL_MATCH;
        reasons.push("all chars valid z85, length multiple of 5".to_string());
    } else if ratio > 0.9 {
        confidence = util::confidence::WEAK_MATCH;
        reasons.push(format!("{:.0}% valid z85 chars", ratio * 100.0));
    }

    DetectCandidate {
        codec: "z85".to_string(),
        confidence: confidence.min(1.0),
        reasons,
        warnings: vec![],
    }
}

pub struct Ascii85;

impl Codec for Ascii85 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "ascii85",
            aliases: &["base85"],
            alphabet: ASCII85_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Ascii85/Base85 encoding (Adobe variant)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(encode_ascii85(input))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_ascii85(input, mode)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_ascii85(input)
    }
}

pub struct Z85;

impl Codec for Z85 {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "z85",
            aliases: &[],
            alphabet: Z85_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Z85 encoding (ZeroMQ RFC 32)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        encode_z85(input)
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        decode_z85(input, mode)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_z85(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii85_encode() {
        let encoded = Ascii85.encode(b"Hello").unwrap();
        assert!(!encoded.is_empty());
        let decoded = Ascii85.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn test_ascii85_decode() {
        let encoded = Ascii85.encode(b"Test").unwrap();
        assert_eq!(Ascii85.decode(&encoded, Mode::Strict).unwrap(), b"Test");
    }

    #[test]
    fn test_ascii85_roundtrip() {
        let data = b"The quick brown fox jumps";
        let encoded = Ascii85.encode(data).unwrap();
        let decoded = Ascii85.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_ascii85_empty() {
        assert_eq!(Ascii85.encode(&[]).unwrap(), "");
        assert_eq!(Ascii85.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_ascii85_zeros() {
        assert_eq!(Ascii85.encode(&[0, 0, 0, 0]).unwrap(), "z");
        assert_eq!(Ascii85.decode("z", Mode::Strict).unwrap(), vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_ascii85_wrapper() {
        let encoded = Ascii85.encode(b"Test").unwrap();
        let wrapped = format!("<~{}~>", encoded);
        let decoded = Ascii85.decode(&wrapped, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Test");
    }

    #[test]
    fn test_ascii85_partial_block() {
        let encoded = Ascii85.encode(b"Hi").unwrap();
        let decoded = Ascii85.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Hi");
    }

    #[test]
    fn test_ascii85_lenient_whitespace() {
        let encoded = Ascii85.encode(b"Test").unwrap();
        let with_space = format!("{} ", encoded);
        let decoded = Ascii85.decode(&with_space, Mode::Lenient).unwrap();
        assert_eq!(decoded, b"Test");
    }

    #[test]
    fn test_z85_encode() {
        assert_eq!(Z85.encode(&[0x86, 0x4F, 0xD2, 0x6F]).unwrap(), "HelloWorld"[..5].to_string());
    }

    #[test]
    fn test_z85_roundtrip() {
        let data = [0x86, 0x4F, 0xD2, 0x6F, 0xB5, 0x59, 0xF7, 0x5B];
        let encoded = Z85.encode(&data).unwrap();
        let decoded = Z85.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_z85_empty() {
        assert_eq!(Z85.encode(&[]).unwrap(), "");
        assert_eq!(Z85.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_z85_invalid_input_length() {
        assert!(Z85.encode(&[1, 2, 3]).is_err());
    }

    #[test]
    fn test_z85_invalid_encoded_length() {
        assert!(Z85.decode("Hell", Mode::Strict).is_err());
    }

    #[test]
    fn test_z85_known_vector() {
        let input = [0x8E, 0x0B, 0xDD, 0x69, 0x76, 0x28, 0xB9, 0x1D];
        let encoded = Z85.encode(&input).unwrap();
        assert_eq!(encoded.len(), 10);
        let decoded = Z85.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_z85_lenient_whitespace() {
        let data = [0x86, 0x4F, 0xD2, 0x6F, 0xB5, 0x59, 0xF7, 0x5B];
        let encoded = Z85.encode(&data).unwrap();
        let with_space = format!("{} ", encoded);
        let decoded = Z85.decode(&with_space, Mode::Lenient).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_detect_ascii85_wrapper() {
        let candidate = detect_ascii85("<~87cURD]i~>");
        assert!(candidate.confidence >= 0.9);
    }

    #[test]
    fn test_detect_z85_valid() {
        let data = [0x86, 0x4F, 0xD2, 0x6F, 0xB5, 0x59, 0xF7, 0x5B];
        let encoded = Z85.encode(&data).unwrap();
        let candidate = detect_z85(&encoded);
        assert!(candidate.confidence >= 0.4);
    }
}
