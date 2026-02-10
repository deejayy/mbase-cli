use crate::io::read_input;
use mbase::error::Result;
use mbase::types::{Context, InputSource, Mode};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DecodeResult {
    pub codec: String,
    pub input: String,
    pub output_length: usize,
    pub output_hex: String,
    pub output_text: Option<String>,
    pub multibase_prefix: Option<char>,
}

#[derive(Debug, Serialize)]
pub struct DecodeAllResult {
    pub input: String,
    pub results: Vec<DecodeCodecResult>,
}

#[derive(Debug, Serialize)]
pub struct DecodeCodecResult {
    pub codec: String,
    pub output_length: Option<usize>,
    pub output_hex: Option<String>,
    pub output_text: Option<String>,
    pub error: Option<String>,
}

pub fn run_decode(ctx: &Context, codec_name: &str, input: &InputSource, mode: Mode, multibase: bool) -> Result<Vec<u8>> {
    let data = read_input(input)?;
    let text = String::from_utf8_lossy(&data);

    if multibase && !text.is_empty() {
        let prefix = text.chars().next().unwrap();
        for meta in ctx.registry.list() {
            if meta.multibase_code == Some(prefix) {
                let codec = ctx.registry.get(meta.name)?;
                return codec.decode(&text[prefix.len_utf8()..], mode);
            }
        }
    }

    let codec = ctx.registry.get(codec_name)?;
    codec.decode(&text, mode)
}

pub fn run_decode_json(ctx: &Context, codec_name: &str, input: &InputSource, mode: Mode, multibase: bool) -> Result<DecodeResult> {
    let data = read_input(input)?;
    let text = String::from_utf8_lossy(&data);
    let input_str = text.trim().to_string();

    let (decoded, multibase_prefix, actual_codec) = if multibase && !text.is_empty() {
        let prefix = text.chars().next().unwrap();
        let mut found = false;
        let mut result = Vec::new();
        let mut detected_codec = codec_name.to_string();

        for meta in ctx.registry.list() {
            if meta.multibase_code == Some(prefix) {
                let codec = ctx.registry.get(meta.name)?;
                result = codec.decode(&text[prefix.len_utf8()..], mode)?;
                detected_codec = meta.name.to_string();
                found = true;
                break;
            }
        }

        if found {
            (result, Some(prefix), detected_codec)
        } else {
            let codec = ctx.registry.get(codec_name)?;
            (codec.decode(&text, mode)?, None, codec_name.to_string())
        }
    } else {
        let codec = ctx.registry.get(codec_name)?;
        (codec.decode(&text, mode)?, None, codec_name.to_string())
    };

    let output_length = decoded.len();
    let output_hex = decoded.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    let output_text = std::str::from_utf8(&decoded)
        .ok()
        .filter(|s| s.chars().all(|c| c == '\n' || c == '\r' || c == '\t' || !c.is_control()))
        .map(String::from);

    Ok(DecodeResult {
        codec: actual_codec,
        input: input_str,
        output_length,
        output_hex,
        output_text,
        multibase_prefix,
    })
}

pub fn run_decode_all_json(ctx: &Context, input: &InputSource, mode: Mode) -> Result<DecodeAllResult> {
    let data = read_input(input)?;
    let text = String::from_utf8_lossy(&data);
    let input_str = text.trim().to_string();
    let mut results = Vec::new();

    for meta in ctx.registry.list() {
        let codec = ctx.registry.get(meta.name)?;
        match codec.decode(&text, mode) {
            Ok(decoded) => {
                let output_hex = decoded.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                let output_text = std::str::from_utf8(&decoded)
                    .ok()
                    .filter(|s| s.chars().all(|c| c == '\n' || c == '\r' || c == '\t' || !c.is_control()))
                    .map(String::from);
                results.push(DecodeCodecResult {
                    codec: meta.name.to_string(),
                    output_length: Some(decoded.len()),
                    output_hex: Some(output_hex),
                    output_text,
                    error: None,
                });
            }
            Err(e) => {
                results.push(DecodeCodecResult {
                    codec: meta.name.to_string(),
                    output_length: None,
                    output_hex: None,
                    output_text: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(DecodeAllResult { input: input_str, results })
}

pub fn run_decode_all(ctx: &Context, input: &InputSource, mode: Mode) -> Result<()> {
    let data = read_input(input)?;
    let text = String::from_utf8_lossy(&data);

    println!("{:<18} DECODED (as text, or hex if binary)", "CODEC");
    println!("{}", "-".repeat(70));

    let mut successes = 0;
    for meta in ctx.registry.list() {
        let codec = ctx.registry.get(meta.name)?;
        if let Ok(decoded) = codec.decode(&text, mode) {
            successes += 1;
            let display = format_decoded(&decoded);
            println!("{:<18} {}", meta.name, display);
        }
    }

    if successes == 0 {
        println!("(no codec could decode the input)");
    }

    Ok(())
}

fn format_decoded(data: &[u8]) -> String {
    if data.is_empty() {
        return "(empty)".to_string();
    }

    let is_valid_text = std::str::from_utf8(data)
        .ok()
        .filter(|s| s.chars().all(|c| c == '\n' || c == '\r' || c == '\t' || !c.is_control()))
        .is_some();

    if is_valid_text {
        let s = String::from_utf8_lossy(data);
        if s.len() > 50 {
            format!("\"{}...\"", &s[..47])
        } else {
            format!("\"{}\"", s)
        }
    } else {
        let hex: String = data.iter().take(25).map(|b| format!("{:02x}", b)).collect();
        if data.len() > 25 {
            format!("[{}...] ({} bytes)", hex, data.len())
        } else {
            format!("[{}] ({} bytes)", hex, data.len())
        }
    }
}
