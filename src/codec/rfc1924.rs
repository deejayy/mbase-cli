use crate::error::{MbaseError as Error, Result};

pub const RFC1924_ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~";
pub const RFC1924_ENCODED_LEN: usize = 20;
pub const RFC1924_BYTES_LEN: usize = 16;

pub fn encode_u128(num: u128) -> String {
    let alphabet = RFC1924_ALPHABET.as_bytes();
    let mut result = Vec::with_capacity(RFC1924_ENCODED_LEN);
    let mut n = num;

    for _ in 0..RFC1924_ENCODED_LEN {
        let digit = (n % 85) as usize;
        result.push(alphabet[digit]);
        n /= 85;
    }

    result.reverse();
    String::from_utf8(result).unwrap()
}

pub fn decode_u128(input: &str) -> Result<u128> {
    if input.len() != RFC1924_ENCODED_LEN {
        return Err(Error::invalid_input(format!(
            "RFC1924 encoding must be exactly {} characters, got {}",
            RFC1924_ENCODED_LEN,
            input.len()
        )));
    }

    let mut num: u128 = 0;

    for (pos, c) in input.chars().enumerate() {
        let digit = RFC1924_ALPHABET
            .chars()
            .position(|x| x == c)
            .ok_or_else(|| Error::InvalidCharacter { char: c, position: pos })? as u128;

        num = num * 85 + digit;
    }

    Ok(num)
}

pub fn bytes_to_u128(bytes: &[u8]) -> Result<u128> {
    if bytes.len() != RFC1924_BYTES_LEN {
        return Err(Error::invalid_input(format!("RFC1924 requires exactly {} bytes, got {}", RFC1924_BYTES_LEN, bytes.len())));
    }

    let mut num: u128 = 0;
    for &byte in bytes {
        num = (num << 8) | (byte as u128);
    }
    Ok(num)
}

pub fn u128_to_bytes(num: u128) -> [u8; RFC1924_BYTES_LEN] {
    let mut bytes = [0u8; RFC1924_BYTES_LEN];
    for i in (0..RFC1924_BYTES_LEN).rev() {
        bytes[RFC1924_BYTES_LEN - 1 - i] = ((num >> (i * 8)) & 0xFF) as u8;
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfc1924_roundtrip_u128() {
        let test_values = vec![0u128, 1u128, 85u128, 255u128, u128::MAX, 0x1080_0000_0000_0000_0008_0800_200C_417A];

        for val in test_values {
            let encoded = encode_u128(val);
            assert_eq!(encoded.len(), RFC1924_ENCODED_LEN);
            let decoded = decode_u128(&encoded).unwrap();
            assert_eq!(decoded, val, "roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_rfc1924_bytes_conversion() {
        let bytes = [
            0x10, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x08, 0x00, 0x20, 0x0C, 0x41, 0x7A,
        ];
        let num = bytes_to_u128(&bytes).unwrap();
        let back = u128_to_bytes(num);
        assert_eq!(back, bytes);
    }

    #[test]
    fn test_rfc1924_spec_example() {
        let bytes = [
            0x10, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x08, 0x00, 0x20, 0x0C, 0x41, 0x7A,
        ];
        let num = bytes_to_u128(&bytes).unwrap();
        let encoded = encode_u128(num);
        assert_eq!(encoded, "4)+k&C#VzJ4br>0wv%Yp");
    }

    #[test]
    fn test_rfc1924_decode_spec_example() {
        let decoded = decode_u128("4)+k&C#VzJ4br>0wv%Yp").unwrap();
        let bytes = u128_to_bytes(decoded);
        assert_eq!(bytes, [0x10, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x08, 0x00, 0x20, 0x0C, 0x41, 0x7A]);
    }

    #[test]
    fn test_rfc1924_wrong_length() {
        assert!(decode_u128("too_short").is_err());
        assert!(decode_u128("way_too_long_for_rfc1924").is_err());
        assert!(bytes_to_u128(&[1, 2, 3]).is_err());
        assert!(bytes_to_u128(&[0u8; 20]).is_err());
    }

    #[test]
    fn test_rfc1924_invalid_char() {
        let mut valid = encode_u128(12345);
        valid.replace_range(10..11, ",");
        assert!(decode_u128(&valid).is_err());
    }

    #[test]
    fn test_rfc1924_all_zeros() {
        let encoded = encode_u128(0);
        assert_eq!(encoded, "00000000000000000000");
        assert_eq!(decode_u128(&encoded).unwrap(), 0);
    }

    #[test]
    fn test_rfc1924_all_ones() {
        let encoded = encode_u128(u128::MAX);
        let decoded = decode_u128(&encoded).unwrap();
        assert_eq!(decoded, u128::MAX);
    }
}
