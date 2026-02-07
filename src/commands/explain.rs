use serde::Serialize;

use mbase::error::{MbaseError, Result};
use crate::io::read_input;
use mbase::types::{Context, InputSource, Mode};

#[derive(Debug, Serialize)]
pub struct ExplainResult {
    pub schema_version: u32,
    pub codec: String,
    pub input_preview: String,
    pub valid: bool,
    pub error: Option<ExplainError>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExplainError {
    pub message: String,
    pub position: Option<usize>,
    pub offending_char: Option<char>,
    pub context: Option<String>,
}

fn get_context(input: &str, pos: usize, window: usize) -> String {
    let start = pos.saturating_sub(window);
    let end = (pos + window + 1).min(input.len());
    let slice = &input[start..end];
    
    let marker_pos = pos - start;
    let mut result = String::new();
    result.push_str(slice);
    result.push('\n');
    for _ in 0..marker_pos {
        result.push(' ');
    }
    result.push('^');
    result
}

fn suggest_fixes(error: &MbaseError, codec_name: &str, input: &str) -> Vec<String> {
    let mut suggestions = Vec::new();

    match error {
        MbaseError::InvalidCharacter { char: c, position: _ } => {
            if c.is_ascii_whitespace() {
                suggestions.push("Try --mode lenient to ignore whitespace".to_string());
            }
            if c.is_ascii_uppercase() || c.is_ascii_lowercase() {
                suggestions.push("Try --mode lenient for case flexibility".to_string());
            }
            if *c == '=' {
                suggestions.push(format!(
                    "Padding character found; try a padded variant like {}pad",
                    codec_name.trim_end_matches("pad")
                ));
            }
        }
        MbaseError::InvalidPadding { .. } => {
            if codec_name.contains("pad") {
                suggestions.push("Input may have incorrect padding; try --mode lenient".to_string());
            } else {
                suggestions.push(format!("Try {}pad variant for padded input", codec_name));
            }
        }
        MbaseError::InvalidLength { expected, actual, .. } => {
            use mbase::error::LengthConstraint;
            match expected {
                LengthConstraint::MultipleOf(2) if codec_name.contains("16") => {
                    suggestions.push("Hex input has odd length; may be missing a character".to_string());
                }
                LengthConstraint::MultipleOf(4) | LengthConstraint::MultipleOf(5) => {
                    suggestions.push(format!("Input length {} doesn't match codec requirements", actual));
                }
                _ => {}
            }
        }
        MbaseError::ChecksumMismatch => {
            suggestions.push("Checksum validation failed; data may be corrupted".to_string());
            suggestions.push("Verify the input was copied correctly".to_string());
        }
        _ => {}
    }

    if input.starts_with("0x") || input.starts_with("0X") {
        suggestions.push("Input has 0x prefix; try --mode lenient or remove prefix".to_string());
    }

    suggestions
}

pub fn run_explain(ctx: &Context, input: InputSource, codec: &str, mode: Mode) -> Result<ExplainResult> {
    let data = read_input(&input)?;
    let text = String::from_utf8_lossy(&data);
    let trimmed = text.trim();

    let codec_impl = ctx.registry.get(codec)?;

    let preview = if trimmed.len() > 60 {
        format!("{}...", &trimmed[..60])
    } else {
        trimmed.to_string()
    };

    let result = match codec_impl.decode(trimmed, mode) {
        Ok(_) => ExplainResult {
            schema_version: 1,
            codec: codec.to_string(),
            input_preview: preview,
            valid: true,
            error: None,
            suggestions: vec![],
        },
        Err(e) => {
            let (position, offending_char, context) = match &e {
                MbaseError::InvalidCharacter { char: c, position: p } => {
                    (Some(*p), Some(*c), Some(get_context(trimmed, *p, 10)))
                }
                _ => (None, None, None),
            };

            let suggestions = suggest_fixes(&e, codec, trimmed);

            ExplainResult {
                schema_version: 1,
                codec: codec.to_string(),
                input_preview: preview,
                valid: false,
                error: Some(ExplainError {
                    message: e.to_string(),
                    position,
                    offending_char,
                    context,
                }),
                suggestions,
            }
        }
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_valid() {
        let ctx = Context::default();
        let result = run_explain(
            &ctx,
            InputSource::Literal(b"SGVsbG8".to_vec()),
            "base64",
            Mode::Strict,
        ).unwrap();
        assert!(result.valid);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_explain_invalid_char() {
        let ctx = Context::default();
        let result = run_explain(
            &ctx,
            InputSource::Literal(b"SGVsbG8!".to_vec()),
            "base64",
            Mode::Strict,
        ).unwrap();
        assert!(!result.valid);
        assert!(result.error.is_some());
        let err = result.error.unwrap();
        assert!(err.offending_char == Some('!'));
    }

    #[test]
    fn test_explain_suggestions() {
        let ctx = Context::default();
        let result = run_explain(
            &ctx,
            InputSource::Literal(b"SGVs bG8".to_vec()),
            "base64",
            Mode::Strict,
        ).unwrap();
        assert!(!result.valid);
        assert!(!result.suggestions.is_empty());
        assert!(result.suggestions.iter().any(|s| s.contains("lenient")));
    }

    #[test]
    fn test_get_context() {
        let input = "Hello World Test";
        let ctx = get_context(input, 6, 5);
        assert!(ctx.contains("World"));
        assert!(ctx.contains("^"));
    }
}
