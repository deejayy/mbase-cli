use std::fs::File;
use std::io::{self, IsTerminal, Write};

use crate::error::Result;
use crate::types::OutputDest;

pub struct OutputConfig {
    pub dest: OutputDest,
    pub force: bool,
}

pub fn write_output(data: &[u8], config: &OutputConfig) -> Result<()> {
    match &config.dest {
        OutputDest::File(path) => {
            let mut file = File::create(path)?;
            file.write_all(data)?;
            Ok(())
        }
        OutputDest::Stdout => {
            let stdout = io::stdout();
            if stdout.is_terminal() && !config.force && !is_safe_for_terminal(data) {
                print_hex_preview(data);
            } else {
                let mut handle = stdout.lock();
                handle.write_all(data)?;
            }
            Ok(())
        }
    }
}

fn is_safe_for_terminal(data: &[u8]) -> bool {
    std::str::from_utf8(data).is_ok()
}

fn print_hex_preview(data: &[u8]) {
    const BYTES_PER_LINE: usize = 16;
    const MAX_LINES: usize = 32;

    let total_lines = data.len().div_ceil(BYTES_PER_LINE);
    let truncated = total_lines > MAX_LINES;
    let lines_to_show = total_lines.min(MAX_LINES);

    eprintln!("Binary output ({} bytes). Showing hex preview (use --force to output raw or --out @file):\n", data.len());

    for line_idx in 0..lines_to_show {
        let offset = line_idx * BYTES_PER_LINE;
        let chunk = &data[offset..(offset + BYTES_PER_LINE).min(data.len())];

        print!("{:08x}  ", offset);

        for (i, byte) in chunk.iter().enumerate() {
            if i == 8 {
                print!(" ");
            }
            print!("{:02x} ", byte);
        }

        for _ in chunk.len()..BYTES_PER_LINE {
            print!("   ");
            if chunk.len() <= 8 && chunk.len() + (BYTES_PER_LINE - chunk.len()) > 8 {
                print!(" ");
            }
        }
        if chunk.len() <= 8 {
            print!(" ");
        }

        print!(" |");
        for byte in chunk {
            let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            };
            print!("{}", ch);
        }
        println!("|");
    }

    if truncated {
        eprintln!("\n... ({} more bytes)", data.len() - (MAX_LINES * BYTES_PER_LINE));
    }
}
