use crate::io::read_input;
use mbase::error::Result;
use mbase::types::{Context, InputSource, Mode};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ConvertResult {
    pub from_codec: String,
    pub to_codec: String,
    pub input: String,
    pub output: String,
}

pub fn run_conv(ctx: &Context, from_codec: &str, to_codec: &str, input: &InputSource, mode: Mode) -> Result<String> {
    let decoder = ctx.registry.get(from_codec)?;
    let encoder = ctx.registry.get(to_codec)?;

    let data = read_input(input)?;
    let text = String::from_utf8_lossy(&data);
    let decoded = decoder.decode(&text, mode)?;
    encoder.encode(&decoded)
}

pub fn run_conv_json(ctx: &Context, from_codec: &str, to_codec: &str, input: &InputSource, mode: Mode) -> Result<ConvertResult> {
    let decoder = ctx.registry.get(from_codec)?;
    let encoder = ctx.registry.get(to_codec)?;

    let data = read_input(input)?;
    let text = String::from_utf8_lossy(&data);
    let input_str = text.trim().to_string();
    let decoded = decoder.decode(&text, mode)?;
    let output = encoder.encode(&decoded)?;

    Ok(ConvertResult {
        from_codec: from_codec.to_string(),
        to_codec: to_codec.to_string(),
        input: input_str,
        output,
    })
}
