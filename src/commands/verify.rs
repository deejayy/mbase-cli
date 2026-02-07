use serde::Serialize;

use crate::io::read_input;
use mbase::error::Result;
use mbase::types::{Context, InputSource, Mode};

#[derive(Debug, Serialize)]
pub struct VerifyResult {
    pub schema_version: u32,
    pub valid: bool,
    pub codec: String,
    pub error: Option<String>,
}

pub fn run_verify(ctx: &Context, codec_name: &str, input: &InputSource, mode: Mode) -> Result<VerifyResult> {
    let codec = ctx.registry.get(codec_name)?;

    let data = read_input(input)?;
    let text = String::from_utf8_lossy(&data);

    match codec.validate(&text, mode) {
        Ok(()) => Ok(VerifyResult {
            schema_version: 1,
            valid: true,
            codec: codec_name.to_string(),
            error: None,
        }),
        Err(e) => Ok(VerifyResult {
            schema_version: 1,
            valid: false,
            codec: codec_name.to_string(),
            error: Some(e.to_string()),
        }),
    }
}
