// copyright 2017 Kaz Wesley

use cryptonight::Hashstate;
use job::{Hash, JobBlob, Nonce};

// XXX TODO: Hasher should be !Send to ensure memory locality correctness

// produces an iter, to support "pipelined" impl
/*
pub trait Hasher: Iterator<Item = (Nonce, Hash)> {
    fn hashes(&mut self, blob) -> Hashes;
}
*/

pub struct AesniPipelinedHasher {
    base_nonce: Nonce,
    state: Hashstate,
}

/// a hasher in-between hash streams
impl AesniPipelinedHasher {
    pub fn new(base_nonce: Nonce) -> Self {
        Self {
            base_nonce,
            state: Hashstate::new().unwrap(),
        }
    }

    pub fn hashes(&mut self, blob: JobBlob) -> Hashes {
        Hashes::new(self.base_nonce, blob, &mut self.state)
    }
}

pub struct Hashes<'a> {
    nonce: Nonce,
    blob: JobBlob,
    state: &'a mut Hashstate,
}

impl<'a> Hashes<'a> {
    fn new(nonce: Nonce, mut blob: JobBlob, state: &'a mut Hashstate) -> Self {
        blob.set_nonce(nonce);
        state.init(blob.as_slice());
        Hashes { nonce, blob, state }
    }
}

impl<'a> Iterator for Hashes<'a> {
    type Item = (Nonce, Hash);

    fn next(&mut self) -> Option<Self::Item> {
        let prev_nonce = self.nonce;
        self.nonce.inc();
        self.blob.set_nonce(self.nonce);
        let prev_result = Hash::new(self.state.advance(self.blob.as_slice()));
        Some((prev_nonce, prev_result))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Config {
    #[serde(rename = "cn-cpu-aesni")] CnCpuAesni { multi: usize },
}

#[derive(Clone)]
pub struct HasherBuilder {}

impl HasherBuilder {
    pub fn new() -> Self {
        HasherBuilder {}
    }

    pub fn into_hasher(self, cfg: &Config, base_nonce: Nonce) -> AesniPipelinedHasher {
        AesniPipelinedHasher::new(base_nonce)
    }
}
