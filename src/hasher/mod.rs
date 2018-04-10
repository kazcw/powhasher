// copyright 2017 Kaz Wesley

use cryptonight::Hashstate;
use job::Hash;

// XXX TODO: Hasher should be !Send to ensure memory locality correctness

/// Number of hashes to do in a batch, i.e. between checks for new work.
const SINGLEHASH_BATCH_SIZE: usize = 16;

pub struct AesniPipelinedHasher {
    base_nonce: u32,
    state: Hashstate,
}

/// a hasher in-between hash streams
impl AesniPipelinedHasher {
    pub fn new(base_nonce: u32) -> Self {
        Self {
            base_nonce,
            state: Hashstate::new().unwrap(),
        }
    }

    pub fn hashes(&mut self, blob: Vec<u8>) -> Hashes {
        Hashes::new(self.base_nonce, blob, &mut self.state)
    }
}

pub struct Hashes<'a> {
    nonce: u32,
    blob: Vec<u8>,
    state: &'a mut Hashstate,
}

impl<'a> Hashes<'a> {
    fn set_nonce(v: &mut [u8], nonce: u32) {
        v[39] = (nonce >> 0x18) as u8;
        v[40] = (nonce >> 0x10) as u8;
        v[41] = (nonce >> 0x08) as u8;
        v[42] = (nonce >> 0x00) as u8;
    }

    fn new(nonce: u32, mut blob: Vec<u8>, state: &'a mut Hashstate) -> Self {
        Self::set_nonce(&mut blob, nonce);
        state.init(&blob);
        Hashes { nonce, blob, state }
    }

    /// Hasher takes over control flow for a batch because:
    /// - pipelined hashers are incompatible with a simple 1-input -> 1-output interface
    /// - hashers/hasher configs have different batch size constraints
    #[inline]
    pub fn run_batch<F>(&mut self, handler: &mut F) -> usize
    where
        F: FnMut(u32, &Hash),
    {
        for _ in 0..SINGLEHASH_BATCH_SIZE {
            let prev_nonce = self.nonce;
            self.nonce = self.nonce.wrapping_add(1);
            Self::set_nonce(&mut self.blob, self.nonce);
            let prev_result = Hash::new(self.state.advance(&self.blob));
            handler(prev_nonce, &prev_result);
        }
        SINGLEHASH_BATCH_SIZE
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Config {
    #[serde(rename = "cn-cpu-aesni")]
    CnCpuAesni { multi: usize },
}

#[derive(Clone)]
pub struct HasherBuilder {}

impl HasherBuilder {
    pub fn new() -> Self {
        HasherBuilder {}
    }

    pub fn into_hasher(self, cfg: &Config, base_nonce: u32) -> AesniPipelinedHasher {
        AesniPipelinedHasher::new(base_nonce)
    }
}
