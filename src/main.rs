mod cli;
mod commands;
mod io;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command};
use commands::CommandHandler;
use mbase::{error, types, Context};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            e.exit_code().into()
        }
    }
}

fn run(cli: Cli) -> error::Result<()> {
    let ctx = Context::default();

    let handler: Box<dyn CommandHandler> = match cli.command {
        Command::Enc {
            codec,
            r#in,
            out,
            multibase,
            all,
        } => Box::new(commands::EncCommand {
            codec,
            input: types::InputSource::parse(&r#in),
            output: types::OutputDest::parse(&out),
            multibase,
            all,
        }),

        Command::Dec {
            codec,
            r#in,
            out,
            mode,
            force,
            multibase,
            all,
        } => Box::new(commands::DecCommand {
            codec,
            input: types::InputSource::parse(&r#in),
            output: types::OutputDest::parse(&out),
            mode: mode.into(),
            force,
            multibase,
            all,
        }),

        Command::Conv { from, to, r#in, out, mode } => Box::new(commands::ConvCommand {
            from,
            to,
            input: types::InputSource::parse(&r#in),
            output: types::OutputDest::parse(&out),
            mode: mode.into(),
        }),

        Command::List { json } => Box::new(commands::ListCommand { json }),

        Command::Info { codec, json } => Box::new(commands::InfoCommand { codec, json }),

        Command::Verify { codec, r#in, mode, json } => Box::new(commands::VerifyCommand {
            codec,
            input: types::InputSource::parse(&r#in),
            mode: mode.into(),
            json,
        }),

        Command::Fmt {
            codec,
            r#in,
            out,
            mode,
            wrap,
            group,
            sep,
        } => Box::new(commands::FmtCommand {
            codec,
            input: types::InputSource::parse(&r#in),
            output: types::OutputDest::parse(&out),
            mode: mode.into(),
            wrap,
            group,
            sep,
        }),

        Command::Detect { r#in, json, top } => Box::new(commands::DetectCommand {
            input: types::InputSource::parse(&r#in),
            json,
            top,
        }),

        Command::Explain { codec, r#in, mode, json } => Box::new(commands::ExplainCommand {
            codec,
            input: types::InputSource::parse(&r#in),
            mode: mode.into(),
            json,
        }),
    };

    handler.execute(&ctx)
}
