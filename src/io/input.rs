use std::fs::File;
use std::io::{self, Read};

use crate::error::Result;
use crate::types::InputSource;

pub fn read_input(source: &InputSource) -> Result<Vec<u8>> {
    match source {
        InputSource::Stdin => {
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf)?;
            Ok(buf)
        }
        InputSource::File(path) => {
            let mut file = File::open(path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            Ok(buf)
        }
        InputSource::Literal(data) => Ok(data.clone()),
    }
}
