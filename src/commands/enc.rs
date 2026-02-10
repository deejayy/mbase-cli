use crate::io::read_input;
use mbase::error::Result;
use mbase::types::{Context, InputSource};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct EncodeResult {
    pub codec: String,
    pub input_length: usize,
    pub output: String,
    pub output_length: usize,
    pub multibase_prefix: Option<char>,
}

#[derive(Debug, Serialize)]
pub struct EncodeAllResult {
    pub input_length: usize,
    pub results: Vec<EncodeCodecResult>,
}

#[derive(Debug, Serialize)]
pub struct EncodeCodecResult {
    pub codec: String,
    pub output: Option<String>,
    pub error: Option<String>,
}

pub fn run_encode(ctx: &Context, codec_name: &str, input: &InputSource, multibase: bool) -> Result<String> {
    let codec = ctx.registry.get(codec_name)?;
    let data = read_input(input)?;
    let encoded = codec.encode(&data)?;

    if multibase {
        if let Some(prefix) = codec.meta().multibase_code {
            return Ok(format!("{}{}", prefix, encoded));
        }
    }

    Ok(encoded)
}

pub fn run_encode_all(ctx: &Context, input: &InputSource) -> Result<String> {
    let data = read_input(input)?;
    let mut output = String::new();

    output.push_str(&format!("{:<18} ENCODED\n", "CODEC"));
    output.push_str(&format!("{}\n", "-".repeat(70)));

    for meta in ctx.registry.list() {
        let codec = ctx.registry.get(meta.name)?;
        match codec.encode(&data) {
            Ok(encoded) => {
                let display = if encoded.len() > 50 {
                    format!("{}...", &encoded[..47])
                } else {
                    encoded
                };
                output.push_str(&format!("{:<18} {}\n", meta.name, display));
            }
            Err(_) => {
                output.push_str(&format!("{:<18} (encoding failed)\n", meta.name));
            }
        }
    }

    Ok(output)
}

pub fn run_encode_json(ctx: &Context, codec_name: &str, input: &InputSource, multibase: bool) -> Result<EncodeResult> {
    let codec = ctx.registry.get(codec_name)?;
    let data = read_input(input)?;
    let input_length = data.len();
    let encoded = codec.encode(&data)?;

    let (output, multibase_prefix) = if multibase {
        if let Some(prefix) = codec.meta().multibase_code {
            (format!("{}{}", prefix, encoded), Some(prefix))
        } else {
            (encoded, None)
        }
    } else {
        (encoded, None)
    };

    let output_length = output.len();

    Ok(EncodeResult {
        codec: codec_name.to_string(),
        input_length,
        output,
        output_length,
        multibase_prefix,
    })
}

pub fn run_encode_all_json(ctx: &Context, input: &InputSource) -> Result<EncodeAllResult> {
    let data = read_input(input)?;
    let input_length = data.len();
    let mut results = Vec::new();

    for meta in ctx.registry.list() {
        let codec = ctx.registry.get(meta.name)?;
        match codec.encode(&data) {
            Ok(encoded) => {
                results.push(EncodeCodecResult {
                    codec: meta.name.to_string(),
                    output: Some(encoded),
                    error: None,
                });
            }
            Err(e) => {
                results.push(EncodeCodecResult {
                    codec: meta.name.to_string(),
                    output: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(EncodeAllResult { input_length, results })
}
