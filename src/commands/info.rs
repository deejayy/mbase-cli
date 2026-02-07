use mbase::error::Result;
use mbase::types::{CodecMeta, Context};

pub fn run_info(ctx: &Context, codec_name: &str) -> Result<CodecMeta> {
    let codec = ctx.registry.get(codec_name)?;
    Ok(codec.meta())
}
