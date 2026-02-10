mod conv;
mod dec;
mod detect;
mod enc;
mod explain;
mod fmt;
mod info;
mod list;
mod verify;

pub use conv::{run_conv, run_conv_json};
pub use dec::{run_decode, run_decode_all, run_decode_all_json, run_decode_json};
pub use detect::run_detect;
pub use enc::{run_encode, run_encode_all, run_encode_all_json, run_encode_json};
pub use explain::run_explain;
pub use fmt::{run_fmt, FmtOptions};
pub use info::run_info;
pub use list::run_list;
pub use verify::run_verify;

use crate::io::{write_output, OutputConfig};
use mbase::error::Result;
use mbase::types::{Context, InputSource, Mode, OutputDest};

pub trait CommandHandler {
    fn execute(&self, ctx: &Context) -> Result<()>;
}

pub struct EncCommand {
    pub codec: String,
    pub input: InputSource,
    pub output: OutputDest,
    pub multibase: bool,
    pub all: bool,
    pub json: bool,
}

impl CommandHandler for EncCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        if self.json {
            if self.all {
                let result = run_encode_all_json(ctx, &self.input)?;
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            } else {
                let result = run_encode_json(ctx, &self.codec, &self.input, self.multibase)?;
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            return Ok(());
        }

        if self.all {
            let output_str = run_encode_all(ctx, &self.input)?;
            let config = OutputConfig {
                dest: self.output.clone(),
                force: true,
            };
            write_output(output_str.as_bytes(), &config)?;
            if matches!(self.output, OutputDest::Stdout) {
                println!();
            }
            return Ok(());
        }

        let encoded = run_encode(ctx, &self.codec, &self.input, self.multibase)?;
        let config = OutputConfig {
            dest: self.output.clone(),
            force: true,
        };
        write_output(encoded.as_bytes(), &config)?;
        if matches!(self.output, OutputDest::Stdout) {
            println!();
        }
        Ok(())
    }
}

pub struct DecCommand {
    pub codec: String,
    pub input: InputSource,
    pub output: OutputDest,
    pub mode: Mode,
    pub force: bool,
    pub multibase: bool,
    pub all: bool,
    pub json: bool,
}

impl CommandHandler for DecCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        if self.json {
            if self.all {
                let result = run_decode_all_json(ctx, &self.input, self.mode)?;
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            } else {
                let result = run_decode_json(ctx, &self.codec, &self.input, self.mode, self.multibase)?;
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            return Ok(());
        }

        if self.all {
            run_decode_all(ctx, &self.input, self.mode)?;
            return Ok(());
        }

        let decoded = run_decode(ctx, &self.codec, &self.input, self.mode, self.multibase)?;
        let config = OutputConfig {
            dest: self.output.clone(),
            force: self.force,
        };
        write_output(&decoded, &config)?;
        Ok(())
    }
}

pub struct ConvCommand {
    pub from: String,
    pub to: String,
    pub input: InputSource,
    pub output: OutputDest,
    pub mode: Mode,
    pub json: bool,
}

impl CommandHandler for ConvCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        if self.json {
            let result = run_conv_json(ctx, &self.from, &self.to, &self.input, self.mode)?;
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
            return Ok(());
        }

        let converted = run_conv(ctx, &self.from, &self.to, &self.input, self.mode)?;
        let config = OutputConfig {
            dest: self.output.clone(),
            force: true,
        };
        write_output(converted.as_bytes(), &config)?;
        if matches!(self.output, OutputDest::Stdout) {
            println!();
        }
        Ok(())
    }
}

pub struct ListCommand {
    pub json: bool,
}

impl CommandHandler for ListCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        let codecs = run_list(ctx);
        if self.json {
            println!("{}", serde_json::to_string_pretty(&codecs).unwrap());
        } else {
            println!("{:<20} {:<8} DESCRIPTION", "NAME", "PREFIX");
            println!("{}", "-".repeat(60));
            for c in codecs {
                let prefix = c.multibase_code.map_or("-".to_string(), |c| c.to_string());
                println!("{:<20} {:<8} {}", c.name, prefix, c.description);
            }
        }
        Ok(())
    }
}

pub struct InfoCommand {
    pub codec: String,
    pub json: bool,
}

impl CommandHandler for InfoCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        let meta = run_info(ctx, &self.codec)?;
        if self.json {
            println!("{}", serde_json::to_string_pretty(&meta).unwrap());
        } else {
            println!("Name:        {}", meta.name);
            println!("Aliases:     {}", meta.aliases.join(", "));
            println!("Alphabet:    {}", meta.alphabet);
            println!("Multibase:   {}", meta.multibase_code.map_or("-".to_string(), |c| c.to_string()));
            println!("Padding:     {:?}", meta.padding);
            println!("Case:        {:?}", meta.case_sensitivity);
            println!("Description: {}", meta.description);
        }
        Ok(())
    }
}

pub struct VerifyCommand {
    pub codec: String,
    pub input: InputSource,
    pub mode: Mode,
    pub json: bool,
}

impl CommandHandler for VerifyCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        let result = run_verify(ctx, &self.codec, &self.input, self.mode)?;
        if self.json {
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        } else if result.valid {
            println!("valid");
        } else {
            println!("invalid: {}", result.error.as_deref().unwrap_or_default());
            return Err(mbase::error::MbaseError::invalid_input(result.error.unwrap_or_default()));
        }
        Ok(())
    }
}

pub struct FmtCommand {
    pub codec: String,
    pub input: InputSource,
    pub output: OutputDest,
    pub mode: Mode,
    pub wrap: Option<usize>,
    pub group: Option<usize>,
    pub sep: String,
}

impl CommandHandler for FmtCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        let opts = FmtOptions {
            wrap: self.wrap,
            group: self.group,
            separator: self.sep.clone(),
        };
        let formatted = run_fmt(ctx, &self.codec, &self.input, self.mode, &opts)?;
        let config = OutputConfig {
            dest: self.output.clone(),
            force: true,
        };
        write_output(formatted.as_bytes(), &config)?;
        if matches!(self.output, OutputDest::Stdout) {
            println!();
        }
        Ok(())
    }
}

pub struct DetectCommand {
    pub input: InputSource,
    pub json: bool,
    pub top: usize,
}

impl CommandHandler for DetectCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        let result = run_detect(ctx, self.input.clone(), self.top)?;

        if self.json {
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        } else {
            println!("Input: {}", result.input_preview);
            println!();
            if result.candidates.is_empty() {
                println!("No likely codecs detected.");
            } else {
                println!("{:<16} {:<8} REASONS", "CODEC", "CONF");
                println!("{}", "-".repeat(60));
                for c in &result.candidates {
                    let conf = format!("{:.0}%", c.confidence * 100.0);
                    let reasons = c.reasons.join("; ");
                    println!("{:<16} {:<8} {}", c.codec, conf, reasons);
                    for w in &c.warnings {
                        println!("{:>16} warning: {}", "", w);
                    }
                }
            }
        }
        Ok(())
    }
}

pub struct ExplainCommand {
    pub codec: String,
    pub input: InputSource,
    pub mode: Mode,
    pub json: bool,
}

impl CommandHandler for ExplainCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        let result = run_explain(ctx, self.input.clone(), &self.codec, self.mode)?;

        if self.json {
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        } else {
            println!("Codec: {}", result.codec);
            println!("Input: {}", result.input_preview);
            println!();

            if result.valid {
                println!("Status: VALID");
                println!("The input is valid for this codec.");
            } else if let Some(ref err) = result.error {
                println!("Status: INVALID");
                println!();
                println!("Error: {}", err.message);

                if let Some(pos) = err.position {
                    println!("Position: {}", pos);
                }
                if let Some(c) = err.offending_char {
                    println!("Character: {:?}", c);
                }
                if let Some(ref context) = err.context {
                    println!();
                    println!("{}", context);
                }

                if !result.suggestions.is_empty() {
                    println!();
                    println!("Suggestions:");
                    for suggestion in &result.suggestions {
                        println!("  - {}", suggestion);
                    }
                }
            }
        }
        Ok(())
    }
}
