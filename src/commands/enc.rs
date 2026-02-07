use mbase::error::Result;
use crate::io::read_input;
use mbase::types::{Context, InputSource};

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

pub fn run_encode_all(ctx: &Context, input: &InputSource) -> Result<()> {
    let data = read_input(input)?;

    println!("{:<18} {}", "CODEC", "ENCODED");
    println!("{}", "-".repeat(70));

    for meta in ctx.registry.list() {
        let codec = ctx.registry.get(meta.name)?;
        match codec.encode(&data) {
            Ok(encoded) => {
                let display = if encoded.len() > 50 {
                    format!("{}...", &encoded[..47])
                } else {
                    encoded
                };
                println!("{:<18} {}", meta.name, display);
            }
            Err(_) => {
                println!("{:<18} (encoding failed)", meta.name);
            }
        }
    }

    Ok(())
}
