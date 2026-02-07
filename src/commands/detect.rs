use serde::Serialize;

use mbase::error::Result;
use crate::io::read_input;
use mbase::types::{Context, DetectCandidate, InputSource, Mode};

#[derive(Debug, Serialize)]
pub struct DetectResult {
    pub schema_version: u32,
    pub candidates: Vec<DetectCandidate>,
    pub input_preview: String,
}

fn detect_multibase_prefix<'a>(input: &str, multibase_map: &'a std::collections::HashMap<char, &'static str>) -> Option<(&'a str, char)> {
    if input.is_empty() {
        return None;
    }
    let first = input.chars().next()?;
    multibase_map.get(&first).map(|&name| (name, first))
}

pub fn run_detect(ctx: &Context, input: InputSource, top_n: usize) -> Result<DetectResult> {
    let data = read_input(&input)?;
    let text = String::from_utf8_lossy(&data);
    let trimmed = text.trim();

    let multibase_map = ctx.registry.multibase_map();
    let mut candidates: Vec<DetectCandidate> = Vec::new();

    if let Some((codec_name, code)) = detect_multibase_prefix(trimmed, &multibase_map) {
        let mut candidate = DetectCandidate {
            codec: codec_name.to_string(),
            confidence: 0.98,
            reasons: vec![format!("multibase prefix '{}' detected", code)],
            warnings: vec![],
        };

        if let Ok(codec) = ctx.registry.get(codec_name) {
            let without_prefix = &trimmed[1..];
            if codec.validate(without_prefix, Mode::Lenient).is_ok() {
                candidate.confidence = 1.0;
                candidate.reasons.push("valid after removing prefix".to_string());
            }
        }
        candidates.push(candidate);
    }

    for codec in ctx.registry.list() {
        let codec_impl = ctx.registry.get(codec.name).unwrap();
        let mut score = codec_impl.detect_score(trimmed);

        if candidates.iter().any(|c| c.codec == score.codec && c.confidence > score.confidence) {
            continue;
        }

        if codec_impl.decode(trimmed, Mode::Lenient).is_ok() {
            if score.confidence < 0.5 {
                score.confidence = 0.5;
            }
            if !score.reasons.iter().any(|r| r.contains("decode")) {
                score.reasons.push("decodes successfully".to_string());
            }
        }

        if score.confidence > 0.0 {
            if let Some(existing) = candidates.iter_mut().find(|c| c.codec == score.codec) {
                if score.confidence > existing.confidence {
                    *existing = score;
                }
            } else {
                candidates.push(score);
            }
        }
    }

    candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    candidates.truncate(top_n);

    let preview = if trimmed.len() > 60 {
        format!("{}...", &trimmed[..60])
    } else {
        trimmed.to_string()
    };

    let result = DetectResult {
        schema_version: 1,
        candidates,
        input_preview: preview,
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_multibase_prefix() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert('z', "base58btc");
        map.insert('m', "base64");
        
        assert_eq!(detect_multibase_prefix("zHello", &map), Some(("base58btc", 'z')));
        assert_eq!(detect_multibase_prefix("mSGVsbG8", &map), Some(("base64", 'm')));
        assert_eq!(detect_multibase_prefix("Hello", &map), None);
    }

    #[test]
    fn test_detect_base64() {
        let ctx = Context::default();
        let result = run_detect(
            &ctx,
            InputSource::Literal(b"SGVsbG8gV29ybGQ".to_vec()),
            5,
        ).unwrap();
        assert!(!result.candidates.is_empty());
        assert!(result.candidates.iter().any(|c| c.codec.contains("base64")));
    }

    #[test]
    fn test_detect_multibase_input() {
        let ctx = Context::default();
        let result = run_detect(
            &ctx,
            InputSource::Literal(b"zJxF12TrwUP45BMd".to_vec()),
            5,
        ).unwrap();
        assert!(!result.candidates.is_empty());
        assert_eq!(result.candidates[0].codec, "base58btc");
        assert!(result.candidates[0].confidence >= 0.95);
    }

    #[test]
    fn test_detect_hex() {
        let ctx = Context::default();
        let result = run_detect(
            &ctx,
            InputSource::Literal(b"f48656c6c6f".to_vec()),
            5,
        ).unwrap();
        assert!(!result.candidates.is_empty());
        assert_eq!(result.candidates[0].codec, "base16lower");
    }
}
