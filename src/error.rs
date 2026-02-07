use std::process::ExitCode as StdExitCode;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    Success = 0,
    GeneralError = 1,
    InvalidInput = 10,
    ChecksumMismatch = 11,
    IoError = 12,
    UnsupportedCodec = 13,
}

impl From<ExitCode> for StdExitCode {
    fn from(code: ExitCode) -> Self {
        StdExitCode::from(code as u8)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LengthConstraint {
    Exact(usize),
    MultipleOf(usize),
    Range { min: usize, max: Option<usize> },
}

impl std::fmt::Display for LengthConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LengthConstraint::Exact(n) => write!(f, "exactly {}", n),
            LengthConstraint::MultipleOf(n) => write!(f, "multiple of {}", n),
            LengthConstraint::Range { min, max: Some(max) } => write!(f, "between {} and {}", min, max),
            LengthConstraint::Range { min, max: None } => write!(f, "at least {}", min),
        }
    }
}

#[derive(Debug, Error)]
pub enum MbaseError {
    #[error("invalid input: {message}")]
    InvalidInput { message: String },

    #[error("invalid character '{char}' at position {position}")]
    InvalidCharacter { char: char, position: usize },

    #[error("invalid length: expected {expected}, got {actual}{}", if !.message.is_empty() { format!(" ({})", .message) } else { String::new() })]
    InvalidLength {
        expected: LengthConstraint,
        actual: usize,
        message: String,
    },

    #[error("invalid padding: {message}")]
    InvalidPadding { message: String },

    #[error("checksum mismatch")]
    ChecksumMismatch,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unsupported codec: {name}")]
    UnsupportedCodec { name: String },
}

impl MbaseError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            MbaseError::InvalidInput { .. }
            | MbaseError::InvalidCharacter { .. }
            | MbaseError::InvalidLength { .. }
            | MbaseError::InvalidPadding { .. } => ExitCode::InvalidInput,
            MbaseError::ChecksumMismatch => ExitCode::ChecksumMismatch,
            MbaseError::Io(_) => ExitCode::IoError,
            MbaseError::UnsupportedCodec { .. } => ExitCode::UnsupportedCodec,
        }
    }

    // Helper constructors for common error patterns
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    pub fn invalid_char(ch: char, pos: usize) -> Self {
        Self::InvalidCharacter {
            char: ch,
            position: pos,
        }
    }

    pub fn invalid_length(expected: LengthConstraint, actual: usize) -> Self {
        Self::InvalidLength {
            expected,
            actual,
            message: String::new(),
        }
    }

    pub fn invalid_length_msg(expected: LengthConstraint, actual: usize, message: impl Into<String>) -> Self {
        Self::InvalidLength {
            expected,
            actual,
            message: message.into(),
        }
    }

    pub fn invalid_padding(message: impl Into<String>) -> Self {
        Self::InvalidPadding {
            message: message.into(),
        }
    }

    pub fn unsupported_codec(name: impl Into<String>) -> Self {
        Self::UnsupportedCodec {
            name: name.into(),
        }
    }
}

// Backward compatibility: allow old tuple-struct syntax to still work
impl From<String> for MbaseError {
    fn from(message: String) -> Self {
        Self::InvalidInput { message }
    }
}

pub type Result<T> = std::result::Result<T, MbaseError>;
