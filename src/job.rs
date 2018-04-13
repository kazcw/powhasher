// copyright 2017 Kaz Wesley

//! Data structures common to poolclient and hasher

use arrayvec::ArrayString;
use generic_array::GenericArray;
use hexbytes;
use serde::Deserializer;
use std::str;
use typenum::U32;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobBlob(#[serde(deserialize_with = "hexbytes::hex_to_varbyte")] pub Vec<u8>);

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct CpuId(pub u8);

#[derive(Debug, Serialize, Clone, Copy)]
pub struct Nonce(#[serde(serialize_with = "hexbytes::u32_to_hex_padded")] pub u32);

impl From<CpuId> for Nonce {
    fn from(cpu: CpuId) -> Self {
        Nonce(u32::from(cpu.0) << 24)
    }
}

#[derive(Debug, Serialize)]
#[repr(align(64))]
pub struct Hash(#[serde(serialize_with = "hexbytes::byte32_to_hex")] [u8; 32]);

impl Hash {
    pub fn new(value: GenericArray<u8, U32>) -> Self {
        use std::convert::TryFrom;
        Hash(*<&[u8; 32]>::try_from(value.as_slice()).unwrap())
    }

    /// extract little-endian high qword
    fn lehigh64(&self) -> u64 {
        let mut val = 0u64;
        for (i, x) in self.0[24..].iter().enumerate() {
            val |= u64::from(*x) << (i * 8);
        }
        val
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct JobId(ArrayString<[u8; 64]>);

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Target(#[serde(deserialize_with = "deserialize_target")] u64);

// Input is either 32-bit or 64-bit little-endian hex string, not necessarily padded.
// Inputs of 8 hex chars or less are in a compact format.
pub fn deserialize_target<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let (mut val, hexlen) = hexbytes::hex64le_to_int(deserializer)?;
    // unpack compact format
    // XXX: this is what other miners do. It doesn't seem right...
    if hexlen <= 8 {
        val |= val << 0x20;
    }
    Ok(val)
}

impl Target {
    pub fn is_hit(&self, hash: &Hash) -> bool {
        hash.lehigh64() <= self.0
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Job {
    pub blob: JobBlob,
    pub job_id: JobId,
    pub target: Target,
}

impl PartialEq<Job> for Job {
    fn eq(&self, other: &Job) -> bool {
        self.job_id == other.job_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn des_target_compact() {
        let t: Target = serde_json::from_str("\"74784100\"").unwrap();
        assert_eq!(t.0, 0x41787400417874);
    }
}
