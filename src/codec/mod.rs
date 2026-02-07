mod base16;
mod base32;
mod base32human;
mod base36;
mod base45;
mod base58;
mod base62;
mod base64;
mod base65536;
mod base85;
mod base91;
mod bech32;
mod proquint;
mod quotedprintable;
mod uuencode;
pub mod registry;
pub(crate) mod util;

pub use registry::Registry;

use crate::error::Result;
use crate::types::{CodecMeta, DetectCandidate, Mode};

pub trait Codec: Send + Sync {
    fn meta(&self) -> CodecMeta;
    fn encode(&self, input: &[u8]) -> Result<String>;
    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>>;
    fn detect_score(&self, input: &str) -> DetectCandidate;

    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        self.decode(input, mode)?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        self.meta().name
    }
}
