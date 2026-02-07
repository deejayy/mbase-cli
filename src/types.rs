use serde::Serialize;
use std::path::PathBuf;

use crate::codec::Registry;

pub struct Context {
    pub registry: &'static Registry,
}

impl Context {
    pub fn new(registry: &'static Registry) -> Self {
        Self { registry }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            registry: Registry::global(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Strict,
    Lenient,
}

#[derive(Debug, Clone)]
pub enum InputSource {
    Stdin,
    File(PathBuf),
    Literal(Vec<u8>),
}

impl InputSource {
    pub fn parse(s: &str) -> Self {
        match s {
            "-" => InputSource::Stdin,
            s if s.starts_with('@') => InputSource::File(PathBuf::from(&s[1..])),
            s => {
                // Warn if input looks like a path
                if Self::looks_like_path(s) {
                    eprintln!("Warning: treating '{}' as literal data. Use @{} to read from file.", s, s);
                }
                InputSource::Literal(s.as_bytes().to_vec())
            }
        }
    }

    fn looks_like_path(s: &str) -> bool {
        // Check for path separators
        if s.contains('/') || s.contains('\\') {
            return true;
        }
        // Check for common file extensions
        let extensions = [".txt", ".bin", ".dat", ".json", ".xml", ".csv", ".log"];
        extensions.iter().any(|ext| s.ends_with(ext))
    }
}

#[derive(Debug, Clone)]
pub enum OutputDest {
    Stdout,
    File(PathBuf),
}

impl OutputDest {
    pub fn parse(s: &str) -> Self {
        match s {
            "-" => OutputDest::Stdout,
            s if s.starts_with('@') => OutputDest::File(PathBuf::from(&s[1..])),
            s => OutputDest::File(PathBuf::from(s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PaddingRule {
    None,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CaseSensitivity {
    Sensitive,
    Insensitive,
    Lower,
    Upper,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodecMeta {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub alphabet: &'static str,
    pub multibase_code: Option<char>,
    pub padding: PaddingRule,
    pub case_sensitivity: CaseSensitivity,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectCandidate {
    pub codec: String,
    pub confidence: f64,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}
