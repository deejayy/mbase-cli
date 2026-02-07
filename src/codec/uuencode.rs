use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

fn encode_char(val: u8) -> char {
    if val == 0 {
        '`'
    } else {
        (val + 32) as char
    }
}

fn decode_char(c: char) -> Option<u8> {
    match c {
        '`' => Some(0),
        ' '..='_' => Some(c as u8 - 32),
        _ => None,
    }
}

pub struct Uuencode;

impl Codec for Uuencode {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "uuencode",
            aliases: &["uu"],
            alphabet: " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`",
            multibase_code: None,
            padding: PaddingRule::None,
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "Unix-to-Unix encoding (traditional)",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        if input.is_empty() {
            return Ok(String::new());
        }

        let mut result = String::new();
        for chunk in input.chunks(45) {
            result.push(encode_char(chunk.len() as u8));
            for triple in chunk.chunks(3) {
                let b0 = triple[0];
                let b1 = triple.get(1).copied().unwrap_or(0);
                let b2 = triple.get(2).copied().unwrap_or(0);

                result.push(encode_char(b0 >> 2));
                result.push(encode_char(((b0 & 0x03) << 4) | (b1 >> 4)));
                result.push(encode_char(((b1 & 0x0F) << 2) | (b2 >> 6)));
                result.push(encode_char(b2 & 0x3F));
            }
            result.push('\n');
        }

        Ok(result)
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let lines: Vec<&str> = input.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line = if mode == Mode::Lenient {
                line.trim_end()
            } else {
                *line
            };

            if line.is_empty() {
                continue;
            }

            let chars: Vec<char> = line.chars().collect();
            if chars.is_empty() {
                continue;
            }

            let length = decode_char(chars[0]).ok_or_else(|| MbaseError::InvalidCharacter {
                char: chars[0],
                position: 0,
            })? as usize;

            if length == 0 {
                continue;
            }

            let encoded_chars = &chars[1..];
            let mut line_data = Vec::new();

            for quad in encoded_chars.chunks(4) {
                if quad.len() < 4 {
                    if mode == Mode::Strict {
                        return Err(MbaseError::invalid_input(format!(
                            "incomplete quad at line {}",
                            line_num + 1
                        )));
                    }
                    break;
                }

                let mut vals = [0u8; 4];
                for (i, &c) in quad.iter().enumerate() {
                    vals[i] = decode_char(c).ok_or_else(|| MbaseError::InvalidCharacter {
                        char: c,
                        position: 1 + i,
                    })?;
                }

                line_data.push((vals[0] << 2) | (vals[1] >> 4));
                line_data.push((vals[1] << 4) | (vals[2] >> 2));
                line_data.push((vals[2] << 6) | vals[3]);
            }

            line_data.truncate(length);
            result.extend(line_data);
        }

        Ok(result)
    }

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let lines: Vec<&str> = input.lines().collect();

        if lines.is_empty() {
            return DetectCandidate {
                codec: self.name().to_string(),
                confidence: 0.0,
                reasons: vec!["empty input".to_string()],
                warnings: vec![],
            };
        }

        let mut valid_lines = 0;
        let mut total_lines = 0;

        for line in &lines {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            total_lines += 1;

            let chars: Vec<char> = line.chars().collect();
            if chars.is_empty() {
                continue;
            }

            if let Some(len) = decode_char(chars[0]) {
                if len <= 45 {
                    let expected_encoded = ((len as usize + 2) / 3) * 4 + 1;
                    if chars.len() >= expected_encoded {
                        let all_valid = chars[1..].iter().all(|&c| decode_char(c).is_some());
                        if all_valid {
                            valid_lines += 1;
                        }
                    }
                }
            }
        }

        if total_lines == 0 {
            return DetectCandidate {
                codec: self.name().to_string(),
                confidence: 0.0,
                reasons: vec!["no content lines".to_string()],
                warnings: vec![],
            };
        }

        let ratio = valid_lines as f64 / total_lines as f64;
        let confidence = if ratio > 0.8 { util::confidence::ALPHABET_MATCH } else { ratio * util::confidence::PARTIAL_MATCH };

        DetectCandidate {
            codec: self.name().to_string(),
            confidence,
            reasons: vec![format!("{}/{} valid uuencode lines", valid_lines, total_lines)],
            warnings: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuencode_empty() {
        let codec = Uuencode;
        assert_eq!(codec.encode(b"").unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), b"");
    }

    #[test]
    fn test_uuencode_cat() {
        let codec = Uuencode;
        let encoded = codec.encode(b"Cat").unwrap();
        assert_eq!(encoded, "#0V%T\n");
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Cat");
    }

    #[test]
    fn test_uuencode_hello() {
        let codec = Uuencode;
        let encoded = codec.encode(b"Hello, World!").unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, b"Hello, World!");
    }

    #[test]
    fn test_uuencode_roundtrip() {
        let codec = Uuencode;
        let inputs = [
            b"".to_vec(),
            b"a".to_vec(),
            b"ab".to_vec(),
            b"abc".to_vec(),
            b"Hello".to_vec(),
            b"The quick brown fox".to_vec(),
            (0..=255).collect::<Vec<u8>>(),
        ];
        for input in inputs {
            let encoded = codec.encode(&input).unwrap();
            let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
            assert_eq!(decoded, input);
        }
    }

    #[test]
    fn test_uuencode_multiline() {
        let codec = Uuencode;
        let long_input: Vec<u8> = (0..100).collect();
        let encoded = codec.encode(&long_input).unwrap();
        assert!(encoded.contains('\n'));
        let lines: Vec<&str> = encoded.lines().collect();
        assert!(lines.len() >= 2);
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, long_input);
    }

    #[test]
    fn test_uuencode_backtick_zero() {
        assert_eq!(encode_char(0), '`');
        assert_eq!(decode_char('`'), Some(0));
    }

    #[test]
    fn test_uuencode_detect() {
        let codec = Uuencode;
        let encoded = codec.encode(b"Hello").unwrap();
        let score = codec.detect_score(&encoded);
        assert!(score.confidence > 0.5);
    }
}
