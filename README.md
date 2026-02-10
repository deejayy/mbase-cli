# mbase

Universal base encoder/decoder/converter. Single binary, 54 codecs, zero plugins.

## Features

- **54 codecs** - base2/8/16/32/58/62/64/85/91/92, bech32, morse, IPv6, braille, punycode, and more  
- **Zero dependencies** - single binary, no plugins or external tools  
- **JSON output** - structured data for scripting and automation  
- **Multibase support** - self-describing encoded data with prefixes  
- **Smart detection** - automatically identify unknown encodings  
- **Error explanation** - detailed diagnostics for decode failures  
- **Format normalization** - wrap, group, and clean encoded data  
- **Batch operations** - encode/decode with all codecs at once

## Install

```bash
cargo install --path .
```

## Quick Start

```bash
# Encode to base64
echo "Hello" | mbase enc --codec base64
# SGVsbG8K

# Decode from base64
echo "SGVsbG8K" | mbase dec --codec base64
# Hello

# Convert between encodings
echo "SGVsbG8K" | mbase conv --from base64 --to base58btc
# 2NEpo7TZRhna7vSvL

# Try text encodings
echo "Hello" | mbase enc --codec rot13  # Uryyb
echo "SOS" | mbase enc --codec morse    # ... --- ...
echo "test@example.com" | mbase enc --codec urlencoding  # test%40example.com

# IPv6 compact representation (RFC1924)
printf "::1" | mbase enc --codec ipv6  # 00000000000000000001
printf "2001:db8::1" | mbase enc --codec ipv6  # 9R}vSQ9RqiCv7SR1r(Uz

# List available codecs
mbase list
```

## Examples

```
$ printf "mbase rokz" | mbase enc --all
CODEC              ENCODED
----------------------------------------------------------------------
atbash             nyzhv ilpa
base2              01101101011000100110000101110011011001010010000...
base8              155142141163145040162157153172
base16lower        6d6261736520726f6b7a
base16upper        6D6261736520726F6B7A
base32lower        nvrgc43febzg6232
base32upper        NVRGC43FEBZG6232
base32padlower     nvrgc43febzg6232
base32padupper     NVRGC43FEBZG6232
base32hexlower     dlh62sr541p6uqrq
base32hexupper     DLH62SR541P6UQRQ
base32hexpadlower  dlh62sr541p6uqrq
base32hexpadupper  DLH62SR541P6UQRQ
zbase32            pitgnh5frb3g6454
crockford32        DNH62WV541S6YTVT
base32wordsafe     pitgnh5frb3g6454
base36lower        2c46lmitvvqlkvwa
base36upper        2C46LMITVVQLKVWA
base37             1KBS9ENGBK7NDL18
base45             C$DHECDZC0LEJQD
base58btc          79S9xSNYRQdHDs
base58flickr       79r9XrnxqpChdS
base58check        hDNqPZfwaMymMTXPt2m
base58ripple       f9S9xS4YRQdHD1
base62             2a6j5tU7aIGuBG
base64             bWJhc2Ugcm9reg
base64pad          bWJhc2Ugcm9reg==
base64url          bWJhc2Ugcm9reg
base64urlpad       bWJhc2Ugcm9reg==
base65536          ꉢ陳騠ꝯꁺ
ascii85            D.6ppAKZ#3CO,
z85                zdl{{wGV2iyKb
base85chunked      ZDL__Wgv2IYkB
base85rfc1924      (encoding failed)
base91             ;GH<f,|L3$P]B
base92             #G9OG=jw{)9K0
baudot             11100110010001100101000010010001010110000111110001
bech32             data1d43xzum9ypex76m6qerv4p
bech32m            data1d43xzum9ypex76m649nqsr
braille            ⠍⠃⠁⠎⠑⠀⠗⠕⠅⠵
bubblebabble       xirekd-omelf-enodb-isokz-opulp-yx
ipv6               (encoding failed)
morse              -- -... .- ... . / .-. --- -.- --..
proquint           kujof-kajug-kihob-lanoz-kotup
punycode           mbase rokz
quoted-printable   mbase=20rokz
rot13              zonfr ebxm
rot47              >32D6 C@<K
a1z26              13-2-1-19-5-0-18-15-11-26
rot18              zonfr ebxm
unicode            U+006D U+0062 U+0061 U+0073 U+0065 U+0020 U+007...
tapcode            32 12 11 43 15    42 34 13 55
uuencode           *;6)A<V4@<F]K>@``
urlencoding        mbase%20rokz
```

```
$ printf "79r9XrnxqpChdS" | mbase dec --all
CODEC              DECODED (as text, or hex if binary)
----------------------------------------------------------------------
atbash             "79i9CimcjkXswH"
base37             [5fe6f3de494aa3529d] (9 bytes)
base58btc          [6d82e124341a7c95fec9] (10 bytes)
base58flickr       "mbase rokz"
base58ripple       [01e309d4e7a776ec68ba6d] (11 bytes)
base62             [012f3cfda3c72fd68fdcca] (11 bytes)
z85                [161ef46754dd320a76dc81] (11 bytes)
base85rfc1924      [1621d217a6b9369c26ed80] (11 bytes)
base91             [ea55bb823dc9d8eb7ad7a0] (11 bytes)
base92             [0252d44e7cee1e1f72e79f9d] (12 bytes)
punycode           "79r9xrnxqpchds"
quoted-printable   "79r9XrnxqpChdS"
rot13              "79e9KeakdcPuqF"
rot47              "fhCh)C?IBAr95$"
rot18              "24e4KeakdcPuqF"
urlencoding        "79r9XrnxqpChdS"
```

## Commands

### `enc` - Encode bytes to text
```bash
mbase enc --codec base64 --in data.bin --out encoded.txt
mbase enc --codec base32 --multibase  # Add multibase prefix
mbase enc --all                       # Show all encodings
mbase enc --codec base64 --json       # JSON output
```

### `dec` - Decode text to bytes
```bash
mbase dec --codec base64 --in encoded.txt --out data.bin
mbase dec --multibase                 # Auto-detect from prefix
mbase dec --all                       # Try all codecs
mbase dec --mode lenient              # Ignore whitespace
mbase dec --codec base64 --json       # JSON output with hex
```

### `conv` - Convert between encodings
```bash
mbase conv --from base64 --to base32
mbase conv --from hex --to base58btc --in data.txt
mbase conv --from base64 --to base32 --json  # JSON output
```

### `verify` - Check if input is valid
```bash
mbase verify --codec base64 --in data.txt
mbase verify --codec hex --mode strict
mbase verify --codec base64 --json   # JSON output
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
mbase detect --json                   # JSON output
```

### `explain` - Debug decode failures
```bash
mbase explain --codec base64 --in bad.txt
mbase explain --codec base64 --json  # JSON output
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

### Binary-to-Text Encodings

**Base2:** `base2` (binary)  
**Base8:** `base8` (octal)  
**Base16:** `base16lower` (hex), `base16upper` (HEX)  
**Base32:** `base32lower`, `base32upper`, `base32padlower`, `base32padupper`, `base32hexlower`, `base32hexupper`, `base32hexpadlower`, `base32hexpadupper`  
**Base32 Variants:** `zbase32`, `crockford32` (human-friendly), `base32wordsafe` (z-base-32, avoids similar chars)  
**Base36:** `base36lower`, `base36upper`  
**Base37:** `base37` (base36 + space character)  
**Base45:** `base45` (RFC 9285, QR-code friendly)  
**Base58:** `base58btc` (Bitcoin), `base58flickr`, `base58check` (Bitcoin-style checksum), `base58ripple` (XRP)  
**Base62:** `base62` (0-9A-Za-z)  
**Base64:** `base64`, `base64pad`, `base64url`, `base64urlpad`  
**Base65536:** `base65536` (Unicode, 2 bytes per char)  
**Base85:** `ascii85` (Adobe), `z85` (ZeroMQ), `base85chunked` (4-byte chunks), `base85rfc1924` (RFC1924 big-integer)  
**Base91:** `base91` (highest density printable ASCII)  
**Base92:** `base92` (92 printable ASCII characters)

### Text Encodings & Ciphers

**ROT Ciphers:** `atbash` (A↔Z), `rot13` (letters +13), `rot47` (ASCII !-~), `rot18` (ROT13 + ROT5)  
**Morse & Telegraph:** `morse` (international), `baudot` (ITA2 5-bit telegraph)  
**Position Encodings:** `a1z26` (A=1...Z=26), `tapcode` (Polybius square knock code)  
**Symbolic:** `braille` (Unicode U+2800-U+28FF), `unicode` (U+XXXX code points)  
**Pronounceable:** `proquint` (2 bytes per quint), `bubblebabble` (OpenSSH fingerprint style)

### Internet & Standards

**URL/Email:** `urlencoding` (RFC 3986 percent-encoding), `quoted-printable` (RFC 2045 MIME)  
**Internationalization:** `punycode` (RFC3492 IDN encoding)  
**Bitcoin/Crypto:** `base58btc`, `base58check`, `bech32` (BIP-173), `bech32m` (BIP-350)  
**Network:** `ipv6` (RFC1924 compact IPv6 representation, 128-bit as base85)  
**Legacy:** `uuencode` (Unix-to-Unix)

## More Examples

### IPv6 Address Encoding (RFC1924)

Compact 20-character base85 representation of IPv6 addresses:

```bash
# Loopback address
printf "::1" | mbase enc --codec ipv6
# 00000000000000000001

# Standard IPv6 address
printf "2001:db8::1" | mbase enc --codec ipv6
# 9R}vSQ9RqiCv7SR1r(Uz

# RFC1924 example from spec
printf "1080:0:0:0:8:800:200C:417A" | mbase enc --codec ipv6
# 4)+k&C#VzJ4br>0wv%Yp

# Decode back to canonical IPv6
printf "4)+k&C#VzJ4br>0wv%Yp" | mbase dec --codec ipv6
# 1080::8:800:200c:417a
```

### Letter Position & Tap Code

```bash
# A1Z26: Letter position encoding
echo "HELLO" | mbase enc --codec a1z26
# 8-5-12-12-15

# Tap code (Polybius square knock code)
echo "SOS" | mbase enc --codec tapcode
# 43 34 43
```

### Symbolic Encodings

```bash
# Braille Unicode patterns
printf "HELLO" | mbase enc --codec braille
# ⠓⠑⠇⠇⠕

# Unicode code points
printf "Hi🚀" | mbase enc --codec unicode
# U+0048 U+0069 U+1F680
```

### Pronounceable Encodings

```bash
# Bubble Babble (OpenSSH fingerprint style)
printf "test" | mbase enc --codec bubblebabble
# xitakh-esalg-ox

# Proquint (pronounceable identifiers)
printf "test" | mbase enc --codec proquint
# lidoj-latuh
```

### Telegraph & Historical

```bash
# Baudot code (ITA2 5-bit telegraph)
echo "HELLO" | mbase enc --codec baudot
# 101000000110010100101100000010

# Morse code
echo "HELLO" | mbase enc --codec morse
# .... . .-.. .-.. ---
```

### Internationalization

```bash
# Punycode (IDN encoding for domain names)
printf "münchen" | mbase enc --codec punycode
# mnchen-3ya

# URL encoding
echo "hello world!" | mbase enc --codec urlencoding
# hello%20world%21
```

## Use Cases

**Data interchange:** Convert between encoding schemes without decode/re-encode errors.

**Debugging:** Detect unknown encodings, explain validation failures, verify correctness.

**Formatting:** Normalize encoded data with wrapping, grouping, or whitespace removal.

**Multibase:** Work with self-describing encoded data (prefixed with codec identifier).

**Scripting:** JSON output for programmatic processing of encode/decode operations.

## JSON Output

Many commands support `--json` for structured output:

```bash
# Encode with JSON
$ echo "test" | mbase enc --codec base64 --json
{
  "codec": "base64",
  "input_length": 5,
  "output": "dGVzdAo=",
  "output_length": 8,
  "multibase_prefix": null
}

# Decode with JSON (includes hex representation)
$ echo "dGVzdA" | mbase dec --codec base64 --json
{
  "codec": "base64",
  "input": "dGVzdA",
  "output_length": 4,
  "output_hex": "74657374",
  "output_text": "test",
  "multibase_prefix": null
}

# Convert with JSON
$ echo "dGVzdA" | mbase conv --from base64 --to base32 --json
{
  "from_codec": "base64",
  "to_codec": "base32",
  "input": "dGVzdA",
  "output": "orsxg5a"
}

# Detect with JSON
$ echo "SGVsbG8" | mbase detect --json
{
  "schema_version": 1,
  "candidates": [
    {
      "codec": "base64",
      "confidence": 0.75,
      "reasons": ["all characters valid", "decodes successfully"],
      "warnings": []
    }
  ],
  "input_preview": "SGVsbG8"
}

# List all codecs as JSON
$ mbase list --json
[
  {
    "name": "base64",
    "aliases": ["b64"],
    "multibase_code": "m",
    "description": "Standard base64 encoding"
  }
]
```

Commands supporting `--json`: `enc`, `dec`, `conv`, `list`, `info`, `verify`, `detect`, `explain`

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
