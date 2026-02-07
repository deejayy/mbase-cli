use sha2::{Digest, Sha256};

use super::util;
use super::Codec;
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

const BTC_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const FLICKR_ALPHABET: &str = "123456789abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";

fn detect_base58(input: &str, codec_name: &str, multibase_code: Option<char>, alphabet: &str) -> DetectCandidate {
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();
    let warnings = Vec::new();

    if let Some(code) = multibase_code {
        if input.starts_with(code) {
            confidence = util::confidence::MULTIBASE_MATCH;
            reasons.push(format!("multibase prefix '{}' detected", code));
        }
    }

    let valid = input.chars().filter(|c| alphabet.contains(*c)).count();
    let ratio = valid as f64 / input.len() as f64;

    if ratio == 1.0 {
        confidence = confidence.max(util::confidence::PARTIAL_MATCH);
        reasons.push("all characters in base58 alphabet".to_string());
    } else if ratio > 0.9 {
        confidence = confidence.max(util::confidence::WEAK_MATCH);
        reasons.push(format!("{:.0}% characters valid", ratio * 100.0));
    }

    DetectCandidate {
        codec: codec_name.to_string(),
        confidence: confidence.min(1.0),
        reasons,
        warnings,
    }
}

fn double_sha256(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    Sha256::digest(first).into()
}

pub struct Base58Btc;

impl Codec for Base58Btc {
    fn name(&self) -> &'static str {
        "base58btc"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base58btc",
            aliases: &["base58", "b58"],
            alphabet: BTC_ALPHABET,
            multibase_code: Some('z'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Base58 Bitcoin alphabet",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(bs58::encode(input).with_alphabet(bs58::Alphabet::BITCOIN).into_string())
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);
        bs58::decode(&cleaned)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_vec()
            .map_err(|e| match e {
                bs58::decode::Error::InvalidCharacter { character, index } => MbaseError::InvalidCharacter {
                    char: character,
                    position: index,
                },
                _ => MbaseError::invalid_input(e.to_string()),
            })
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        util::validate_alphabet(input, BTC_ALPHABET, mode)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_base58(input, "base58btc", Some('z'), BTC_ALPHABET)
    }
}

pub struct Base58Flickr;

impl Codec for Base58Flickr {
    fn name(&self) -> &'static str {
        "base58flickr"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base58flickr",
            aliases: &[],
            alphabet: FLICKR_ALPHABET,
            multibase_code: Some('Z'),
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Base58 Flickr alphabet",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        Ok(bs58::encode(input).with_alphabet(bs58::Alphabet::FLICKR).into_string())
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);
        bs58::decode(&cleaned)
            .with_alphabet(bs58::Alphabet::FLICKR)
            .into_vec()
            .map_err(|e| match e {
                bs58::decode::Error::InvalidCharacter { character, index } => MbaseError::InvalidCharacter {
                    char: character,
                    position: index,
                },
                _ => MbaseError::invalid_input(e.to_string()),
            })
    }

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        util::validate_alphabet(input, FLICKR_ALPHABET, mode)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        detect_base58(input, "base58flickr", Some('Z'), FLICKR_ALPHABET)
    }
}

pub struct Base58Check;

impl Codec for Base58Check {
    fn name(&self) -> &'static str {
        "base58check"
    }

    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "base58check",
            aliases: &["b58check"],
            alphabet: BTC_ALPHABET,
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Base58 with 4-byte checksum (Bitcoin-style double-SHA256)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let hash = double_sha256(input);
        let mut with_checksum = input.to_vec();
        with_checksum.extend_from_slice(&hash[..4]);
        Ok(bs58::encode(&with_checksum).with_alphabet(bs58::Alphabet::BITCOIN).into_string())
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let cleaned = util::clean_for_mode(input, mode);
        let decoded = bs58::decode(&cleaned)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_vec()
            .map_err(|e| match e {
                bs58::decode::Error::InvalidCharacter { character, index } => MbaseError::InvalidCharacter {
                    char: character,
                    position: index,
                },
                _ => MbaseError::invalid_input(e.to_string()),
            })?;

        if decoded.len() < 4 {
            return Err(MbaseError::invalid_input("input too short for checksum"));
        }

        let (payload, checksum) = decoded.split_at(decoded.len() - 4);
        let expected = &double_sha256(payload)[..4];

        if checksum != expected {
            return Err(MbaseError::ChecksumMismatch);
        }

        Ok(payload.to_vec())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut candidate = detect_base58(input, "base58check", None, BTC_ALPHABET);

        if self.decode(input, Mode::Lenient).is_ok() {
            candidate.confidence = candidate.confidence.max(0.9);
            candidate.reasons.push("checksum valid".to_string());
        }

        candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base58btc_encode() {
        assert_eq!(Base58Btc.encode(b"Hello World").unwrap(), "JxF12TrwUP45BMd");
    }

    #[test]
    fn test_base58btc_decode() {
        assert_eq!(Base58Btc.decode("JxF12TrwUP45BMd", Mode::Strict).unwrap(), b"Hello World");
    }

    #[test]
    fn test_base58btc_roundtrip() {
        let data = b"The quick brown fox jumps over the lazy dog";
        let encoded = Base58Btc.encode(data).unwrap();
        let decoded = Base58Btc.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base58btc_leading_zeros() {
        let data = b"\x00\x00Hello";
        let encoded = Base58Btc.encode(data).unwrap();
        assert!(encoded.starts_with("11"));
        let decoded = Base58Btc.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base58btc_empty() {
        assert_eq!(Base58Btc.encode(&[]).unwrap(), "");
        assert_eq!(Base58Btc.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_base58btc_lenient_whitespace() {
        assert_eq!(Base58Btc.decode("JxF12 TrwUP 45BMd", Mode::Lenient).unwrap(), b"Hello World");
    }

    #[test]
    fn test_base58btc_invalid_char() {
        let result = Base58Btc.decode("JxF12TrwUP45BMd0", Mode::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn test_base58flickr_encode() {
        let encoded = Base58Flickr.encode(b"Hello").unwrap();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_base58flickr_roundtrip() {
        let data = b"Test data for Flickr";
        let encoded = Base58Flickr.encode(data).unwrap();
        let decoded = Base58Flickr.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base58check_encode_decode() {
        let data = b"Hello";
        let encoded = Base58Check.encode(data).unwrap();
        let decoded = Base58Check.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base58check_roundtrip() {
        let data = b"Bitcoin address payload";
        let encoded = Base58Check.encode(data).unwrap();
        let decoded = Base58Check.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base58check_invalid_checksum() {
        let data = b"Hello";
        let mut encoded = Base58Check.encode(data).unwrap();
        let bytes: Vec<char> = encoded.chars().collect();
        let last = bytes.last().unwrap();
        let replacement = if *last == '1' { '2' } else { '1' };
        encoded.pop();
        encoded.push(replacement);

        let result = Base58Check.decode(&encoded, Mode::Strict);
        assert!(matches!(result, Err(MbaseError::ChecksumMismatch)));
    }

    #[test]
    fn test_base58check_too_short() {
        let result = Base58Check.decode("1", Mode::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn test_base58check_with_version_byte() {
        let mut payload = vec![0x00];
        payload.extend_from_slice(b"test");
        let encoded = Base58Check.encode(&payload).unwrap();
        let decoded = Base58Check.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn test_base58_validate() {
        assert!(Base58Btc.validate("JxF12TrwUP45BMd", Mode::Strict).is_ok());
        assert!(Base58Btc.validate("JxF12TrwUP45BMd0", Mode::Strict).is_err());
    }

    #[test]
    fn test_base58_detect_multibase() {
        let candidate = Base58Btc.detect_score("zJxF12TrwUP45BMd");
        assert!(candidate.confidence >= 0.9);
        assert!(candidate.reasons.iter().any(|r| r.contains("multibase")));
    }
}
