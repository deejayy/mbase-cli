use mbase::error::Result;
use crate::io::read_input;
use mbase::types::{Context, InputSource, Mode};

pub fn run_decode(
    ctx: &Context,
    codec_name: &str,
    input: &InputSource,
    mode: Mode,
    multibase: bool,
) -> Result<Vec<u8>> {
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

pub fn run_decode_all(ctx: &Context, input: &InputSource, mode: Mode) -> Result<()> {
    let data = read_input(input)?;
    let text = String::from_utf8_lossy(&data);

    println!("{:<18} {}", "CODEC", "DECODED (as text, or hex if binary)");
    println!("{}", "-".repeat(70));

    let mut successes = 0;
    for meta in ctx.registry.list() {
        let codec = ctx.registry.get(meta.name)?;
        match codec.decode(&text, mode) {
            Ok(decoded) => {
                successes += 1;
                let display = format_decoded(&decoded);
                println!("{:<18} {}", meta.name, display);
            }
            Err(_) => {}
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

    let is_printable = data.iter().all(|&b| {
        b == b'\n' || b == b'\r' || b == b'\t' || (b >= 0x20 && b < 0x7F)
    });

    if is_printable {
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
