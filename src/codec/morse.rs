use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};
use std::collections::HashMap;

fn morse_table() -> HashMap<char, &'static str> {
    let mut map = HashMap::new();
    map.insert('A', ".-");
    map.insert('B', "-...");
    map.insert('C', "-.-.");
    map.insert('D', "-..");
    map.insert('E', ".");
    map.insert('F', "..-.");
    map.insert('G', "--.");
    map.insert('H', "....");
    map.insert('I', "..");
    map.insert('J', ".---");
    map.insert('K', "-.-");
    map.insert('L', ".-..");
    map.insert('M', "--");
    map.insert('N', "-.");
    map.insert('O', "---");
    map.insert('P', ".--.");
    map.insert('Q', "--.-");
    map.insert('R', ".-.");
    map.insert('S', "...");
    map.insert('T', "-");
    map.insert('U', "..-");
    map.insert('V', "...-");
    map.insert('W', ".--");
    map.insert('X', "-..-");
    map.insert('Y', "-.--");
    map.insert('Z', "--..");
    map.insert('0', "-----");
    map.insert('1', ".----");
    map.insert('2', "..---");
    map.insert('3', "...--");
    map.insert('4', "....-");
    map.insert('5', ".....");
    map.insert('6', "-....");
    map.insert('7', "--...");
    map.insert('8', "---..");
    map.insert('9', "----.");
    map.insert('.', ".-.-.-");
    map.insert(',', "--..--");
    map.insert('?', "..--..");
    map.insert('!', "-.-.--");
    map.insert('/', "-..-.");
    map.insert('@', ".--.-.");
    map.insert('=', "-...-");
    map.insert(' ', "/");
    map
}

fn reverse_morse_table() -> HashMap<&'static str, char> {
    morse_table().into_iter().map(|(k, v)| (v, k)).collect()
}

pub struct Morse;

impl Codec for Morse {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "morse",
            aliases: &["morsecode"],
            alphabet: ".-/ ",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Insensitive,
            description: "International Morse code (space-separated)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        let table = morse_table();
        let text = String::from_utf8_lossy(input).to_uppercase();

        let morse_chars: Vec<&str> = text.chars().filter_map(|c| table.get(&c).copied()).collect();

        if morse_chars.is_empty() && !input.is_empty() {
            return Err(MbaseError::invalid_input("no encodable characters found"));
        }

        Ok(morse_chars.join(" "))
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        let table = reverse_morse_table();
        let cleaned = if mode == Mode::Lenient {
            input.trim().to_string()
        } else {
            input.to_string()
        };

        let mut result = String::new();

        for word in cleaned.split('/') {
            if !result.is_empty() {
                result.push(' ');
            }

            for code in word.split_whitespace() {
                if code.is_empty() {
                    continue;
                }

                let ch = table
                    .get(code)
                    .ok_or_else(|| MbaseError::invalid_input(format!("unknown morse sequence: {}", code)))?;
                result.push(*ch);
            }
        }

        Ok(result.into_bytes())
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        let warnings = Vec::new();

        if input.is_empty() {
            return DetectCandidate {
                codec: "morse".to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let morse_chars = input.chars().filter(|c| matches!(c, '.' | '-' | ' ' | '/')).count();
        let morse_ratio = morse_chars as f64 / input.len() as f64;

        if morse_ratio == 1.0 {
            confidence = util::confidence::ALPHABET_MATCH;
            reasons.push("all characters are morse symbols".to_string());

            let codes: Vec<&str> = input.split_whitespace().collect();
            if codes.iter().all(|code| code.chars().all(|c| c == '.' || c == '-' || c == '/')) {
                reasons.push("valid morse code patterns".to_string());
            }
        } else if morse_ratio > 0.8 {
            confidence = util::confidence::WEAK_MATCH;
            reasons.push(format!("{:.1}% morse characters", morse_ratio * 100.0));
        }

        DetectCandidate {
            codec: "morse".to_string(),
            confidence,
            reasons,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morse_encode() {
        assert_eq!(Morse.encode(b"SOS").unwrap(), "... --- ...");
        assert_eq!(Morse.encode(b"HELLO").unwrap(), ".... . .-.. .-.. ---");
        assert_eq!(Morse.encode(b"A").unwrap(), ".-");
    }

    #[test]
    fn test_morse_decode() {
        assert_eq!(Morse.decode("... --- ...", Mode::Strict).unwrap(), b"SOS");
        assert_eq!(Morse.decode(".... . .-.. .-.. ---", Mode::Strict).unwrap(), b"HELLO");
        assert_eq!(Morse.decode(".-", Mode::Strict).unwrap(), b"A");
    }

    #[test]
    fn test_morse_roundtrip() {
        let data = b"HELLO WORLD";
        let encoded = Morse.encode(data).unwrap();
        assert_eq!(Morse.decode(&encoded, Mode::Strict).unwrap(), data);
    }

    #[test]
    fn test_morse_numbers() {
        assert_eq!(Morse.encode(b"123").unwrap(), ".---- ..--- ...--");
        assert_eq!(Morse.decode(".---- ..--- ...--", Mode::Strict).unwrap(), b"123");
    }

    #[test]
    fn test_morse_punctuation() {
        assert_eq!(Morse.encode(b"HELLO, WORLD!").unwrap(), ".... . .-.. .-.. --- --..-- / .-- --- .-. .-.. -.. -.-.--");
    }

    #[test]
    fn test_morse_with_spaces() {
        let encoded = Morse.encode(b"A B").unwrap();
        assert_eq!(encoded, ".- / -...");
        assert_eq!(Morse.decode(&encoded, Mode::Strict).unwrap(), b"A B");
    }

    #[test]
    fn test_morse_case_insensitive() {
        let upper = Morse.encode(b"HELLO").unwrap();
        let lower = Morse.encode(b"hello").unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn test_morse_invalid_sequence() {
        assert!(Morse.decode(".-.-.-.-", Mode::Strict).is_err()); // Invalid - ambiguous without spaces
        assert!(Morse.decode("xyz", Mode::Strict).is_err()); // Invalid symbols
    }

    #[test]
    fn test_morse_empty() {
        assert_eq!(Morse.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_morse_alphabet() {
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let encoded = Morse.encode(alphabet).unwrap();
        let decoded = Morse.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, alphabet);
    }

    #[test]
    fn test_morse_digits() {
        let digits = b"0123456789";
        let encoded = Morse.encode(digits).unwrap();
        let decoded = Morse.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, digits);
    }
}
