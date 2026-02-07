use crate::io::read_input;
use mbase::error::Result;
use mbase::types::{Context, InputSource, Mode};

pub struct FmtOptions {
    pub wrap: Option<usize>,
    pub group: Option<usize>,
    pub separator: String,
}

impl Default for FmtOptions {
    fn default() -> Self {
        Self {
            wrap: None,
            group: None,
            separator: " ".to_string(),
        }
    }
}

pub fn run_fmt(ctx: &Context, codec_name: &str, input: &InputSource, mode: Mode, opts: &FmtOptions) -> Result<String> {
    let codec = ctx.registry.get(codec_name)?;

    let data = read_input(input)?;
    let text = String::from_utf8_lossy(&data);

    let decoded = codec.decode(&text, mode)?;
    let mut encoded = codec.encode(&decoded)?;

    if let Some(group_size) = opts.group {
        encoded = insert_separators(&encoded, group_size, &opts.separator);
    }

    if let Some(width) = opts.wrap {
        encoded = wrap_lines(&encoded, width);
    }

    Ok(encoded)
}

fn insert_separators(s: &str, group_size: usize, separator: &str) -> String {
    if group_size == 0 {
        return s.to_string();
    }
    s.chars()
        .collect::<Vec<_>>()
        .chunks(group_size)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(separator)
}

fn wrap_lines(s: &str, width: usize) -> String {
    if width == 0 {
        return s.to_string();
    }
    s.chars()
        .collect::<Vec<_>>()
        .chunks(width)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_separators() {
        assert_eq!(insert_separators("ABCDEFGH", 4, " "), "ABCD EFGH");
        assert_eq!(insert_separators("ABCDEFGHI", 4, "-"), "ABCD-EFGH-I");
    }

    #[test]
    fn test_wrap_lines() {
        assert_eq!(wrap_lines("ABCDEFGH", 4), "ABCD\nEFGH");
        assert_eq!(wrap_lines("ABCDEFGHI", 4), "ABCD\nEFGH\nI");
    }
}
