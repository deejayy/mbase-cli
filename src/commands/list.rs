use mbase::types::{CodecMeta, Context};

pub fn run_list(ctx: &Context) -> Vec<CodecMeta> {
    ctx.registry.list()
}
