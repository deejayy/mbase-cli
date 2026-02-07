# mbase

Universal base encoder/decoder/converter. Single binary, 32+ codecs, zero plugins.

## Install

```bash
cargo install --path .
```

## Quick Start

```bash
# Encode
echo "Hello" | mbase enc --codec base64
# SGVsbG8K

# Decode
echo "SGVsbG8K" | mbase dec --codec base64
# Hello

# Convert between encodings
echo "SGVsbG8K" | mbase conv --from base64 --to base58btc
# 2NEpo7TZRhna7vSvL

# List available codecs
mbase list
```

## Commands

### `enc` - Encode bytes to text
```bash
mbase enc --codec base64 --in data.bin --out encoded.txt
mbase enc --codec base32 --multibase  # Add multibase prefix
mbase enc --all                       # Show all encodings
```

### `dec` - Decode text to bytes
```bash
mbase dec --codec base64 --in encoded.txt --out data.bin
mbase dec --multibase                 # Auto-detect from prefix
mbase dec --all                       # Try all codecs
mbase dec --mode lenient              # Ignore whitespace
```

### `conv` - Convert between encodings
```bash
mbase conv --from base64 --to base32
mbase conv --from hex --to base58btc --in data.txt
```

### `verify` - Check if input is valid
```bash
mbase verify --codec base64 --in data.txt
mbase verify --codec hex --mode strict
```

### `fmt` - Normalize/format encoded data
```bash
mbase fmt --codec base64 --wrap 64    # Wrap lines
mbase fmt --codec hex --group 2 --sep :  # AA:BB:CC:DD
```

### `detect` - Identify encoding
```bash
mbase detect --in unknown.txt
mbase detect --top 3                  # Show top 3 candidates
```

### `explain` - Debug decode failures
```bash
mbase explain --codec base64 --in bad.txt
```

### `info` - Show codec details
```bash
mbase info base64
mbase info base58btc --json
```

### `list` - List all codecs
```bash
mbase list
mbase list --json
```

## Supported Codecs

**Base16:** hex, hexupper  
**Base32:** base32, base32upper, base32pad, base32padupper, base32hex, base32hexupper, base32hexpad, base32hexpadupper, zbase32, crockford32  
**Base36:** base36, base36upper  
**Base45:** base45  
**Base58:** base58btc, base58flickr, base58check  
**Base62:** base62  
**Base64:** base64, base64pad, base64url, base64urlpad  
**Base65536:** base65536  
**Base85:** ascii85, z85  
**Base91:** base91  
**Bech32:** bech32, bech32m  
**Other:** proquint, quotedprintable, uuencode

## Use Cases

**Data interchange:** Convert between encoding schemes without decode/re-encode errors.

**Debugging:** Detect unknown encodings, explain validation failures, verify correctness.

**Formatting:** Normalize encoded data with wrapping, grouping, or whitespace removal.

**Multibase:** Work with self-describing encoded data (prefixed with codec identifier).

**Scripting:** JSON output for programmatic processing of encode/decode operations.

## Files & I/O

- `--in` defaults to stdin (`-`)
- `--out` defaults to stdout (`-`)
- Use file paths for non-streaming I/O: `--in data.bin --out result.txt`

## Modes

- **Strict:** Reject invalid input immediately
- **Lenient:** Ignore whitespace and formatting

Default varies by command (`strict` for decode/verify, `lenient` for fmt).

## License

MIT
