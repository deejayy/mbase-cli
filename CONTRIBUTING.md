# Contributing to mbase

Thank you for your interest in contributing to mbase! This guide will help you add new codecs, commands, and understand the codebase architecture.

## Quick Links

- [Adding a New Codec](#adding-a-new-codec) (15 minutes)
- [Adding a New Command](#adding-a-new-command) (30 minutes)
- [Testing Guidelines](#testing-guidelines)
- [Architecture Overview](#architecture-overview)

---

## Adding a New Codec

### Quick Start (3 steps, ~10 minutes)

#### 1. Create Codec Implementation

Create `src/codec/mynew.rs`:

```rust
use super::{util, Codec};
use crate::error::{MbaseError, Result};
use crate::types::{CaseSensitivity, CodecMeta, DetectCandidate, Mode, PaddingRule};

pub struct MyNewCodec;

impl Codec for MyNewCodec {
    fn meta(&self) -> CodecMeta {
        CodecMeta {
            name: "mynew",
            aliases: &["mn", "mynewcodec"],
            alphabet: "0123456789ABCDEF",  // Your alphabet
            multibase_code: Some('x'),      // Optional multibase prefix
            padding: PaddingRule::None,     // Or Required/Optional
            case_sensitivity: CaseSensitivity::Sensitive,
            description: "My new encoding format",
        }
    }

    fn encode(&self, input: &[u8]) -> Result<String> {
        // Your encoding logic here
        Ok(String::new())
    }

    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>> {
        // Clean whitespace according to mode
        let cleaned = util::clean_for_mode(input, mode);
        
        // Your decoding logic here
        Ok(Vec::new())
    }

    // validate() has a default implementation that calls decode()
    // Only override if you need custom validation logic

    fn detect_score(&self, input: &str) -> DetectCandidate {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        
        // Check for multibase prefix
        if input.starts_with('x') {
            confidence = util::confidence::MULTIBASE_MATCH;  // 0.95
            reasons.push("multibase prefix detected".to_string());
        }
        
        // Check alphabet match
        let valid_chars = input.chars()
            .filter(|c| "0123456789ABCDEF".contains(*c))
            .count();
        let ratio = valid_chars as f64 / input.len() as f64;
        
        if ratio == 1.0 {
            confidence = confidence.max(util::confidence::ALPHABET_MATCH);  // 0.70
            reasons.push("all characters valid".to_string());
        } else if ratio >= 0.9 {
            confidence = confidence.max(util::confidence::WEAK_MATCH);  // 0.30
        }
        
        DetectCandidate {
            codec: self.name().to_string(),
            confidence: confidence.min(1.0),
            reasons,
            warnings: vec![],
        }
    }
}
```

#### 2. Register in Module System

Edit `src/codec/mod.rs`:

```rust
// Add to module declarations (around line 15)
mod mynew;

// Codec structs are NOT exported from mod.rs
// They are registered directly in registry.rs
```

#### 3. Register in Registry Macro

Edit `src/codec/registry.rs` in the `register_codecs!` macro invocation (~line 47-80):

```rust
register_codecs! {
    // ... existing codecs ...
    mynew::MyNewCodec,  // Add your codec here (alphabetical order recommended)
}
```

**That's it!** The macro automatically:
- Registers your codec in the global registry
- Builds the name and alias maps
- Detects duplicate multibase codes at compile time
- Generates test expectations

#### 4. Verify

```bash
cargo test
cargo run -- list  # Should see your codec
cargo run -- enc --codec mynew -i "Hello"
cargo run -- detect -i "xYourEncodedData"
```

---

### Codec Implementation Guidelines

#### Use Shared Utilities

The `util` module provides common helpers:

```rust
// Clean whitespace according to mode
let cleaned = util::clean_for_mode(input, mode);

// Validate alphabet (rejects invalid characters)
util::validate_alphabet(input, "0123456789", mode)?;

// With padding support
util::validate_alphabet_with_padding(input, "ABCD", true)?;
```

#### Detection Confidence Constants

Use named constants instead of magic numbers:

```rust
use super::util::confidence;

// Available constants:
confidence::MULTIBASE_MATCH  // 0.95 - Has correct multibase prefix
confidence::ALPHABET_MATCH   // 0.70 - All characters in alphabet
confidence::PARTIAL_MATCH    // 0.50 - Partial match
confidence::WEAK_MATCH       // 0.30 - Weak indicators
```

#### Validation Pattern

Most codecs can use the **default `validate()` implementation** which calls `decode()`:

```rust
// DEFAULT - No need to implement validate()
impl Codec for MyCodec {
    // ... meta, encode, decode ...
    // validate() automatically calls self.decode()
}
```

Only override `validate()` if you need **custom logic**:

```rust
// CUSTOM - When validation differs from decode
fn validate(&self, input: &str, mode: Mode) -> Result<()> {
    util::validate_alphabet(input, MY_ALPHABET, mode)?;
    // Additional checks...
    Ok(())
}
```

#### Error Handling Best Practices

Map external library errors to structured variants:

```rust
use crate::error::MbaseError;

// GOOD - Preserve error context
.map_err(|e| match e {
    ExternalError::InvalidChar { ch, pos } => {
        MbaseError::InvalidCharacter { 
            char: ch, 
            position: pos 
        }
    },
    ExternalError::BadLength(msg) => {
        MbaseError::InvalidLength(msg)
    },
    _ => MbaseError::InvalidInput(e.to_string()),
})

// AVOID - Loses error information
.map_err(|e| MbaseError::InvalidInput(e.to_string()))
```

Available error variants:
- `InvalidInput(String)` - Generic validation error
- `InvalidCharacter { char, position }` - Specific bad character
- `InvalidLength(String)` - Wrong input length
- `ChecksumMismatch` - Checksum validation failed
- `IoError(io::Error)` - File I/O problems

#### Testing Your Codec

Add unit tests in your codec file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let codec = MyNewCodec;
        let data = b"Hello World";
        let encoded = codec.encode(data).unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_empty_input() {
        let codec = MyNewCodec;
        assert_eq!(codec.encode(&[]).unwrap(), "");
        assert_eq!(codec.decode("", Mode::Strict).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_lenient_mode_whitespace() {
        let codec = MyNewCodec;
        let encoded = "AB CD EF";
        let result = codec.decode(encoded, Mode::Lenient);
        assert!(result.is_ok());
    }

    #[test]
    fn test_strict_mode_rejects_whitespace() {
        let codec = MyNewCodec;
        let encoded = "AB CD EF";
        let result = codec.decode(encoded, Mode::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_characters() {
        let codec = MyNewCodec;
        let result = codec.decode("INVALID!", Mode::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn test_multibase_detection() {
        let codec = MyNewCodec;
        let score = codec.detect_score("xABCDEF");
        assert!(score.confidence >= 0.9);
    }
}
```

---

## Adding a New Command

### Quick Start (30 minutes)

#### 1. Create Command Module

Create `src/commands/mynew.rs`:

```rust
use crate::error::Result;
use crate::io::read_input;
use mbase::types::{Context, InputSource};

pub fn run_mynew(ctx: &Context, input: &InputSource) -> Result<String> {
    let data = read_input(input)?;
    
    // Access registry via context
    let codec = ctx.registry.get("base64")?;
    
    // Your command logic here
    let output = String::from_utf8_lossy(&data).to_string();
    
    Ok(output)
}
```

#### 2. Register in Commands Module

Edit `src/commands/mod.rs`:

```rust
mod mynew;
pub use mynew::run_mynew;
```

#### 3. Add CLI Definition

Edit `src/cli.rs`:

```rust
#[derive(Subcommand)]
pub enum Command {
    // ... existing commands ...
    
    #[command(about = "My new command description")]
    Mynew {
        #[arg(long, short = 'i', default_value = "-")]
        r#in: String,
        
        #[arg(long, short = 'o', default_value = "-")]
        out: String,
    },
}
```

#### 4. Create CommandHandler Struct

Edit `src/commands/mod.rs` to add your command struct:

```rust
pub struct MynewCommand {
    pub input: InputSource,
    pub output: OutputDest,
}

impl CommandHandler for MynewCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        let result = run_mynew(ctx, &self.input)?;
        
        let config = OutputConfig {
            dest: self.output.clone(),
            force: true,
        };
        write_output(result.as_bytes(), &config)?;
        Ok(())
    }
}
```

#### 5. Add Command Dispatch

Edit `src/main.rs` in the `run()` function:

```rust
fn run(cli: Cli) -> error::Result<()> {
    let ctx = Context::default();
    
    let handler: Box<dyn CommandHandler> = match cli.command {
        // ... existing commands ...
        
        Command::Mynew { r#in, out } => Box::new(commands::MynewCommand {
            input: types::InputSource::parse(&r#in),
            output: types::OutputDest::parse(&out),
        }),
    };
    
    handler.execute(&ctx)
}
```

#### 5. Add Integration Test

Edit `tests/cli.rs`:

```rust
#[test]
fn test_mynew_command() {
    cmd()
        .arg("mynew")
        .arg("-i").arg("test input")
        .assert()
        .success();
}
```

---

## Testing Guidelines

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test cli

# Run tests for specific module
cargo test codec::base64

# Run with output visible
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'
```

### Test Coverage

Your codec should have tests for:

1. **Roundtrip encoding/decoding** - encode → decode → original data
2. **Empty input** - Both encoding and decoding empty data
3. **Mode handling** - Strict vs Lenient mode behavior
4. **Invalid input** - Proper error handling
5. **Edge cases** - Padding, leading zeros, special characters
6. **Detection** - Confidence scoring works correctly

### Property Testing (Optional)

For robust codecs, consider property-based tests:

```rust
#[test]
fn test_arbitrary_roundtrip() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    for _ in 0..100 {
        let len = rng.gen_range(0..1000);
        let data: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
        
        let codec = MyCodec;
        let encoded = codec.encode(&data).unwrap();
        let decoded = codec.decode(&encoded, Mode::Strict).unwrap();
        assert_eq!(decoded, data);
    }
}
```

---

## Architecture Overview

### Project Structure

```
mbase/
├── src/
│   ├── main.rs              # Entry point, CLI dispatch
│   ├── cli.rs               # Clap CLI definitions
│   ├── error.rs             # Error types, exit codes
│   ├── types.rs             # Core types (Mode, CodecMeta, etc.)
│   ├── codec/
│   │   ├── mod.rs           # Codec trait definition
│   │   ├── registry.rs      # Global codec registry
│   │   ├── util.rs          # Shared codec utilities
│   │   └── *.rs             # Individual codec implementations (18 files)
│   ├── commands/
│   │   ├── mod.rs           # Command exports
│   │   └── *.rs             # Command implementations (9 files)
│   └── io/
│       ├── input.rs         # Input reading (files, stdin, strings)
│       └── output.rs        # Output writing (files, stdout)
└── tests/
    ├── cli.rs               # Integration tests
    └── codec_registration.rs # Registry verification tests
```

### Key Traits and Types

#### Codec Trait

```rust
pub trait Codec: Send + Sync {
    fn meta(&self) -> CodecMeta;
    fn encode(&self, input: &[u8]) -> Result<String>;
    fn decode(&self, input: &str, mode: Mode) -> Result<Vec<u8>>;
    fn detect_score(&self, input: &str) -> DetectCandidate;
    
    // Default implementations:
    fn validate(&self, input: &str, mode: Mode) -> Result<()> {
        self.decode(input, mode)?;
        Ok(())
    }
    fn name(&self) -> &'static str {
        self.meta().name
    }
}
```

#### Mode

```rust
pub enum Mode {
    Strict,   // Reject whitespace, enforce case, strict validation
    Lenient,  // Allow whitespace, case-insensitive, permissive
}
```

#### Error Types

```rust
pub enum MbaseError {
    InvalidInput(String),
    InvalidCharacter { char: char, position: usize },
    InvalidLength(String),
    ChecksumMismatch,
    CodecNotFound(String),
    IoError(io::Error),
    // ... more variants
}
```

### Registry Pattern

The global registry uses a singleton pattern with `OnceLock`:

```rust
static REGISTRY: OnceLock<Registry> = OnceLock::new();

impl Registry {
    pub fn global() -> &'static Registry {
        REGISTRY.get_or_init(Registry::new)
    }
    
    pub fn get(&self, name: &str) -> Result<&dyn Codec> {
        // Lookup by name or alias
    }
}
```

#### Context-Based Dependency Injection

Commands receive the registry via a `Context` struct for testability:

```rust
pub struct Context {
    pub registry: &'static Registry,
}

impl Default for Context {
    fn default() -> Self {
        Self { registry: Registry::global() }
    }
}
```

Used in command implementations:

```rust
pub fn run_encode(ctx: &Context, codec: &str, input: &InputSource) -> Result<String> {
    let codec = ctx.registry.get(codec)?;
    let data = read_input(input)?;
    codec.encode(&data)
}
```

### Command Pattern

Commands use the `CommandHandler` trait for uniform execution:

#### CommandHandler Trait

```rust
pub trait CommandHandler {
    fn execute(&self, ctx: &Context) -> Result<()>;
}
```

#### Command Structure

Each command is a struct implementing `CommandHandler`:

```rust
pub struct DetectCommand {
    pub input: InputSource,
    pub json: bool,
    pub top: usize,
}

impl CommandHandler for DetectCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        // 1. Call business logic function
        let candidates = run_detect(ctx, &self.input, self.top)?;
        
        // 2. Handle output formatting
        if self.json {
            println!("{}", serde_json::to_string_pretty(&candidates)?);
        } else {
            for candidate in candidates {
                println!("{}: {:.0}%", candidate.codec, candidate.confidence * 100.0);
            }
        }
        Ok(())
    }
}
```

#### Separation of Concerns

1. **Business logic** in `src/commands/*.rs` - Pure functions accepting `&Context`
2. **I/O and formatting** in `CommandHandler::execute()` - Handles JSON/text output
3. **Dispatch** in `src/main.rs` - Creates command structs and calls `execute()`

Example flow:

```rust
// Business logic (testable, pure)
pub fn run_detect(ctx: &Context, input: &InputSource, top: usize) -> Result<Vec<DetectCandidate>> {
    let data = read_input(input)?;
    let mut candidates = ctx.registry.list()
        .iter()
        .map(|meta| {
            let codec = ctx.registry.get(meta.name).unwrap();
            codec.detect_score(&data)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    Ok(candidates.into_iter().take(top).collect())
}

// Command handler (I/O, formatting)
impl CommandHandler for DetectCommand {
    fn execute(&self, ctx: &Context) -> Result<()> {
        let candidates = run_detect(ctx, &self.input, self.top)?;
        // ... formatting logic ...
    }
}
```

---

## Development Workflow

### 1. Local Development

```bash
# Build
cargo build

# Run
cargo run -- enc --codec base64 -i "Hello"

# Test
cargo test

# Check for issues
cargo clippy
cargo fmt --check
```

### 2. Before Committing

```bash
# Format code
cargo fmt

# Run all tests
cargo test --all

# Check clippy warnings
cargo clippy --all-targets

# Verify CLI works
cargo run -- list
cargo run -- enc --codec base64 -i "test"
```

### 3. Commit Message Style

Follow conventional commits:

```
feat: add base93 codec support
fix: handle empty input in base64 padding
docs: update CONTRIBUTING with testing guide
refactor: extract validation utility function
test: add roundtrip tests for base58
```

---

## Common Pitfalls

### 1. Forgetting to Register Codec

**Problem:** Codec implementation exists but not registered in `registry.rs`

**Symptom:** `cargo test` fails in `codec_registration` test

**Fix:** Add to registry as described in step 3 above

### 2. Not Handling Empty Input

**Problem:** Codec panics or returns error on empty input

**Best Practice:** Always handle empty input gracefully:

```rust
if input.is_empty() {
    return Ok(Vec::new());  // or Ok(String::new()) for encode
}
```

### 3. Hardcoding Confidence Scores

**Problem:** Using magic numbers like `0.7` instead of named constants

**Fix:** Use `util::confidence::*` constants

### 4. Not Testing Both Modes

**Problem:** Only testing `Mode::Strict`, forgetting `Mode::Lenient`

**Fix:** Add tests for both modes, especially whitespace handling

### 5. Losing Error Context

**Problem:** Converting all errors to generic strings

**Fix:** Map to specific error variants when possible (see Error Handling section)

---

## Getting Help

- **Questions?** Open an issue on GitHub
- **Bug found?** Open an issue with minimal reproduction
- **Feature idea?** Open an issue for discussion first

---

## Code Style

- **Formatting:** Use `cargo fmt` (follows rustfmt defaults)
- **Linting:** Address `cargo clippy` warnings
- **Naming:** Follow Rust conventions (snake_case for functions, PascalCase for types)
- **Comments:** Prefer self-documenting code; use comments for "why" not "what"
- **Tests:** Test names should describe the scenario: `test_empty_input_returns_empty_vec`

---

## License

By contributing, you agree that your contributions will be licensed under the project's license.
