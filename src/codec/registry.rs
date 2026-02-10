use std::collections::HashMap;
use std::sync::OnceLock;

use super::Codec;
use crate::error::{MbaseError, Result};
use crate::types::CodecMeta;

macro_rules! register_codecs {
    ($($module:ident :: $codec:ident),* $(,)?) => {
        fn build_registry() -> Registry {
            let codecs: Vec<Box<dyn Codec>> = vec![
                $(Box::new(super::$module::$codec)),*
            ];

            let mut name_map = HashMap::new();
            for (idx, codec) in codecs.iter().enumerate() {
                name_map.insert(codec.name(), idx);
                for alias in codec.meta().aliases {
                    name_map.insert(*alias, idx);
                }
            }

            let mut multibase_codes: HashMap<char, &str> = HashMap::new();
            for codec in codecs.iter() {
                if let Some(code) = codec.meta().multibase_code {
                    if let Some(existing) = multibase_codes.insert(code, codec.name()) {
                        panic!(
                            "Duplicate multibase code '{}' for codecs '{}' and '{}'",
                            code, existing, codec.name()
                        );
                    }
                }
            }

            Registry { codecs, name_map }
        }

        // Public for testing - generates list of expected codec names
        pub fn expected_codec_names() -> Vec<&'static str> {
            use crate::codec::Codec;
            vec![
                $(super::$module::$codec.name(),)*
            ]
        }
    };
}

register_codecs! {
    atbash::Atbash,
    base2_8::Base2,
    base2_8::Base8,
    base16::Base16Lower,
    base16::Base16Upper,
    base32::Base32Lower,
    base32::Base32Upper,
    base32::Base32PadLower,
    base32::Base32PadUpper,
    base32::Base32HexLower,
    base32::Base32HexUpper,
    base32::Base32HexPadLower,
    base32::Base32HexPadUpper,
    base32human::ZBase32,
    base32human::Crockford32,
    base32wordsafe::Base32WordSafe,
    base36::Base36Lower,
    base36::Base36Upper,
    base37::Base37,
    base45::Base45,
    base58::Base58Btc,
    base58::Base58Flickr,
    base58::Base58Check,
    base58ripple::Base58Ripple,
    base62::Base62,
    base64::Base64,
    base64::Base64Pad,
    base64::Base64Url,
    base64::Base64UrlPad,
    base65536::Base65536,
    base85::Ascii85,
    base85::Z85,
    base85chunked::Base85Chunked,
    base85rfc1924::Base85Rfc1924,
    base91::Base91,
    base92::Base92,
    baudot::Baudot,
    bech32::Bech32Codec,
    bech32::Bech32mCodec,
    braille::Braille,
    bubblebabble::BubbleBabble,
    ipv6::Ipv6,
    morse::Morse,
    proquint::Proquint,
    punycode::Punycode,
    quotedprintable::QuotedPrintable,
    rot::Rot13,
    rot::Rot47,
    simple_text::A1Z26,
    simple_text::Rot18,
    unicode_tap::UnicodeCodepoints,
    unicode_tap::TapCode,
    uuencode::Uuencode,
    urlencoding::UrlEncoding,
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

pub struct Registry {
    codecs: Vec<Box<dyn Codec>>,
    name_map: HashMap<&'static str, usize>,
}

impl Registry {
    fn new() -> Self {
        build_registry()
    }

    pub fn global() -> &'static Registry {
        REGISTRY.get_or_init(Registry::new)
    }

    pub fn get(&self, name: &str) -> Result<&dyn Codec> {
        let name_lower = name.to_lowercase();
        self.name_map
            .get(name_lower.as_str())
            .or_else(|| self.name_map.get(name))
            .map(|&idx| self.codecs[idx].as_ref())
            .ok_or_else(|| MbaseError::unsupported_codec(name))
    }

    pub fn list(&self) -> Vec<CodecMeta> {
        self.codecs.iter().map(|c| c.meta()).collect()
    }

    pub fn multibase_map(&self) -> HashMap<char, &'static str> {
        self.codecs
            .iter()
            .filter_map(|c| {
                let meta = c.meta();
                meta.multibase_code.map(|code| (code, meta.name))
            })
            .collect()
    }
}
