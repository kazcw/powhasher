// copyright 2017 Kaz Wesley

//! Utilities for serde of hex-encoded data

use arrayvec::ArrayVec;
use serde::{self, Deserializer, Serializer};
use std::str;

fn nibble_to_hex(x: u8) -> Result<u8, ()> {
    match x {
        0x0...0x9 => Ok(x - 0x0 + b'0'),
        0xa...0xf => Ok(x - 0xa + b'a'),
        _ => Err(()),
    }
}

fn hex_to_nibble(x: u8) -> Result<u8, ()> {
    match x {
        b'0'...b'9' => Ok(x - b'0' + 0x0),
        b'a'...b'f' => Ok(x - b'a' + 0xa),
        _ => Err(()),
    }
}

pub fn buffer_to_hex_string(buffer: &[u8]) -> String {
    let mut buf = Vec::with_capacity(2 * buffer.len());
    for c in buffer.iter() {
        buf.push(nibble_to_hex((c >> 4) & 0xf as u8).unwrap());
        buf.push(nibble_to_hex(c & 0xf as u8).unwrap());
    }
    String::from_utf8(buf).unwrap()
}

pub fn buffer_to_hex<S>(buffer: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&buffer_to_hex_string(buffer))
}

pub fn byte32_to_hex<S>(buffer: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    buffer_to_hex(buffer.as_ref(), serializer)
}

pub fn u32_to_hex_string_bytes_padded(n: &u32) -> ArrayVec<[u8; 8]> {
    let mut buf = ArrayVec::new();
    for i in 0..4 {
        let x0 = (n >> (8*i+4)) & 0xfu32;
        let x1 = (n >> (8*i)) & 0xfu32;
        buf.push(nibble_to_hex(x0 as u8).unwrap());
        buf.push(nibble_to_hex(x1 as u8).unwrap());
    }
    buf
}

pub fn u32_to_hex_padded<S>(n: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(str::from_utf8(&u32_to_hex_string_bytes_padded(n)).unwrap())
}

pub fn hex_to_varbyte<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let bytes_in = <&str as serde::Deserialize>::deserialize(deserializer)?.as_bytes();
    if bytes_in.len() & 0x1 != 0 {
        return Err(Error::custom("odd-length hex blob"));
    }
    (bytes_in
        .exact_chunks(2)
        .map(|ab| hex_to_nibble(ab[0]).and_then(|a| Ok(a << 4 | hex_to_nibble(ab[1])?)))
        .collect(): Result<Vec<_>, _>)
        .map_err(|_| Error::custom("non-hex char in input"))
}

use serde::de::{self, Visitor};
use std::fmt;
struct Hex64leStrVisitor {}
impl<'de> Visitor<'de> for Hex64leStrVisitor {
    type Value = (u64, usize);

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("hex string")
    }

    fn visit_str<E>(self, hex_in: &str) -> Result<(u64, usize), E>
    where
        E: de::Error,
    {
        use serde::de::Error;
        let hex_in = hex_in.as_bytes();
        let hexlen = hex_in.len();
        if hexlen > 16 {
            return Err(Error::custom("too many input bytes for hex64le"));
        }
        let mut out = 0u64;
        for (i, xs) in hex_in.chunks(2).enumerate() {
            let nib0 = u64::from(hex_to_nibble(xs[0]).map_err(|_| Error::custom("non-hex char in input"))?);
            let nib1 = u64::from(hex_to_nibble(xs[1]).map_err(|_| Error::custom("non-hex char in input"))?);
            out |= nib0 << (i * 8 + 4);
            out |= nib1 << (i * 8);
        }
        Ok((out, hexlen))
    }
}

/// Deserialize a value of up to 64 bits, reporting number of hex bytes it contained
pub fn hex64le_to_int<'de, D>(deserializer: D) -> Result<(u64, usize), D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(Hex64leStrVisitor {})
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[derive(Deserialize)]
    struct Hex64leOut(#[serde(deserialize_with = "hex64le_to_int")] (u64, usize));

    #[test]
    fn hex64le_to_int_test0() {
        let Hex64leOut((val, len)) = serde_json::from_str("\"00\"").unwrap();
        assert_eq!(len, 2);
        assert_eq!(val, 0x0);
    }

    #[test]
    fn hex64le_to_int_test1() {
        let Hex64leOut((val, len)) = serde_json::from_str("\"000102\"").unwrap();
        assert_eq!(len, 6);
        assert_eq!(val, 0x020100);
    }

    #[test]
    fn hex64le_to_int_test2() {
        let Hex64leOut((val, len)) = serde_json::from_str("\"1020304050607080\"").unwrap();
        assert_eq!(len, 16);
        assert_eq!(val, 0x8070605040302010);
    }

    /*
    #[test]
    fn ser_buf_0() {
        let hex = serde_json::to_string(&HexBuf(vec![0u8])).unwrap();
        assert_eq!(hex, "\"00\"");
    }

    #[test]
    fn ser_buf_7_23_42_13() {
        let hex = serde_json::to_string(&HexBuf(vec![0x7u8,0x23u8,0x42u8,0x13u8])).unwrap();
        assert_eq!(hex, "\"07234213\"");
    }
    */
}
