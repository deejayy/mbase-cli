use crate::io::read_input;
use mbase::error::Result;
use mbase::types::{Context, InputSource, Mode};

pub fn run_conv(ctx: &Context, from_codec: &str, to_codec: &str, input: &InputSource, mode: Mode) -> Result<String> {
    let decoder = ctx.registry.get(from_codec)?;
    let encoder = ctx.registry.get(to_codec)?;

    let data = read_input(input)?;
    let text = String::from_utf8_lossy(&data);
    let decoded = decoder.decode(&text, mode)?;
    encoder.encode(&decoded)
}
