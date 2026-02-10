use clap::{Parser, Subcommand, ValueEnum};

use crate::types::Mode;

#[derive(Parser)]
#[command(name = "mbase")]
#[command(about = "Universal base encode/decode/convert CLI")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Encode bytes to text")]
    Enc {
        #[arg(long, default_value = "base64")]
        codec: String,

        #[arg(long, short = 'i', default_value = "-")]
        r#in: String,

        #[arg(long, short = 'o', default_value = "-")]
        out: String,

        #[arg(long, help = "Emit multibase prefix")]
        multibase: bool,

        #[arg(long, help = "Show encoding with all codecs")]
        all: bool,

        #[arg(long, help = "Output as JSON")]
        json: bool,
    },

    #[command(about = "Decode text to bytes")]
    Dec {
        #[arg(long, default_value = "base64")]
        codec: String,

        #[arg(long, short = 'i', default_value = "-")]
        r#in: String,

        #[arg(long, short = 'o', default_value = "-")]
        out: String,

        #[arg(long, default_value = "strict")]
        mode: ModeArg,

        #[arg(long)]
        force: bool,

        #[arg(long, help = "Consume multibase prefix to detect codec")]
        multibase: bool,

        #[arg(long, help = "Try all codecs and show successful decodes")]
        all: bool,

        #[arg(long, help = "Output as JSON")]
        json: bool,
    },

    #[command(about = "Convert between encodings")]
    Conv {
        #[arg(long)]
        from: String,

        #[arg(long)]
        to: String,

        #[arg(long, short = 'i', default_value = "-")]
        r#in: String,

        #[arg(long, short = 'o', default_value = "-")]
        out: String,

        #[arg(long, default_value = "strict")]
        mode: ModeArg,

        #[arg(long, help = "Output as JSON")]
        json: bool,
    },

    #[command(about = "List supported codecs")]
    List {
        #[arg(long)]
        json: bool,
    },

    #[command(about = "Show codec details")]
    Info {
        codec: String,

        #[arg(long)]
        json: bool,
    },

    #[command(about = "Verify input conforms to codec")]
    Verify {
        #[arg(long, default_value = "base64")]
        codec: String,

        #[arg(long, short = 'i', default_value = "-")]
        r#in: String,

        #[arg(long, default_value = "strict")]
        mode: ModeArg,

        #[arg(long)]
        json: bool,
    },

    #[command(about = "Normalize/format encoded data")]
    Fmt {
        #[arg(long, default_value = "base64")]
        codec: String,

        #[arg(long, short = 'i', default_value = "-")]
        r#in: String,

        #[arg(long, short = 'o', default_value = "-")]
        out: String,

        #[arg(long, default_value = "lenient")]
        mode: ModeArg,

        #[arg(long, help = "Wrap output at N characters")]
        wrap: Option<usize>,

        #[arg(long, help = "Group characters with separator")]
        group: Option<usize>,

        #[arg(long, default_value = " ", help = "Separator for grouping")]
        sep: String,
    },

    #[command(about = "Detect likely codec(s) for input")]
    Detect {
        #[arg(long, short = 'i', default_value = "-")]
        r#in: String,

        #[arg(long)]
        json: bool,

        #[arg(long, default_value = "5", help = "Number of candidates to show")]
        top: usize,
    },

    #[command(about = "Explain why input fails to decode")]
    Explain {
        #[arg(long, default_value = "base64")]
        codec: String,

        #[arg(long, short = 'i', default_value = "-")]
        r#in: String,

        #[arg(long, default_value = "strict")]
        mode: ModeArg,

        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ModeArg {
    Strict,
    Lenient,
}

impl From<ModeArg> for Mode {
    fn from(arg: ModeArg) -> Self {
        match arg {
            ModeArg::Strict => Mode::Strict,
            ModeArg::Lenient => Mode::Lenient,
        }
    }
}
