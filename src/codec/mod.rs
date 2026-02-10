mod atbash;
mod base16;
mod base2_8;
mod base32;
mod base32human;
mod base32wordsafe;
mod base36;
mod base37;
mod base45;
mod base58;
mod base58ripple;
mod base62;
mod base64;
mod base65536;
mod base85;
mod base85chunked;
mod base85rfc1924;
mod base91;
mod base92;
mod baudot;
mod bech32;
mod braille;
mod bubblebabble;
mod ipv6;
mod morse;
mod proquint;
mod punycode;
mod quotedprintable;
pub mod registry;
pub(crate) mod rfc1924;
mod rot;
mod simple_text;
mod unicode_tap;
mod urlencoding;
pub(crate) mod util;
mod uuencode;

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
