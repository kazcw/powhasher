// copyright 2017 Kaz Wesley

#![feature(asm)]
#![feature(ptr_internals)]
#![feature(repr_simd)]
#![feature(stdsimd)]
#![feature(type_ascription)]
#![feature(untagged_unions)]

#[macro_use]
extern crate serde_derive;

mod aesni;
mod cn_aesni;
mod mmap;
mod state;

use blake::digest::Digest;
use skein::digest::generic_array::typenum::U32;
use skein::digest::generic_array::GenericArray;
use std::arch::x86_64::__m128i as i64x2;

use self::mmap::Mmap;
use self::state::State;

fn finalize(mut data: State) -> GenericArray<u8, U32> {
    keccak::f1600((&mut data).into());
    let bytes: &[u8; 200] = (&data).into();
    match bytes[0] & 3 {
        0 => blake::Blake256::digest(bytes),
        1 => groestl_aesni::Groestl256::digest(bytes),
        2 => jh_x86_64::Jh256::digest(bytes),
        3 => skein::Skein512::<U32>::digest(bytes),
        _ => unreachable!(),
    }
}

fn read_u64le(bytes: &[u8]) -> u64 {
    u64::from(bytes[0]) | (u64::from(bytes[1]) << 8) | (u64::from(bytes[2]) << 16)
        | (u64::from(bytes[3]) << 24) | (u64::from(bytes[4]) << 32) | (u64::from(bytes[5]) << 40)
        | (u64::from(bytes[6]) << 48) | (u64::from(bytes[7]) << 56)
}

fn set_nonce(blob: &mut [u8], nonce: u32) {
    blob[39] = nonce as u8;
    blob[40] = (nonce >> 0x08) as u8;
    blob[41] = (nonce >> 0x10) as u8;
    blob[42] = (nonce >> 0x18) as u8;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HasherConfig {
    n: u32
}

pub trait Hasher<Noncer> {
    fn set_blob(&mut self, blob: Vec<u8>, noncer: Noncer);
    fn next_hash(&mut self) -> GenericArray<u8, U32>;
}

impl<Noncer> Iterator for Hasher<Noncer> {
    type Item = [u8; 32];
    fn next(&mut self) -> Option<Self::Item> {
        let mut h = [0u8; 32];
        h.copy_from_slice(&self.next_hash());
        Some(h)
    }
}

pub fn hasher<Noncer: Iterator<Item = u32> + 'static>(
    algo: &str,
    cfg: &HasherConfig,
    blob: Vec<u8>,
    noncer: Noncer,
) -> Box<Hasher<Noncer>> {
    match algo {
        "cn/1" => match cfg.n {
            1 => Box::new(CryptoNight::new(blob, noncer)),
            2 => Box::new(CryptoNight2::new(blob, noncer)),
            _ => unimplemented!("unsupported configuration"),
        }
        "cn-lite/0" => match cfg.n {
            1 => Box::new(CryptoNightLite::new(blob, noncer)),
            2 => Box::new(CryptoNightLite2::new(blob, noncer)),
            _ => unimplemented!("unsupported configuration"),
        }
        "cn-lite/1" => match cfg.n {
            1 => Box::new(CryptoNightLiteV1::new(blob, noncer)),
            _ => unimplemented!("unsupported configuration"),
        }
        "cn-heavy" => match cfg.n {
            1 => Box::new(CryptoNightHeavy::new(blob, noncer)),
            _ => unimplemented!("unsupported configuration"),
        }
        "cn/xtl" => match cfg.n {
            1 => Box::new(CryptoNightXtl::new(blob, noncer)),
            _ => unimplemented!("unsupported configuration"),
        }
        _ => unimplemented!("unsupported algo")
    }
}

#[derive(Default)]
pub struct CryptoNight<Noncer> {
    memory: Mmap<[i64x2; 1 << 17]>,
    blob: Vec<u8>,
    state0: State,
    state1: State,
    tweak: u64,
    noncer: Noncer,
}

impl<Noncer: Iterator<Item = u32>> CryptoNight<Noncer> {
    fn transplode(&mut self) -> GenericArray<u8, U32> {
        set_nonce(&mut self.blob, self.noncer.next().unwrap());
        self.state1 = State::from(sha3::Keccak256Full::digest(&self.blob));
        self.tweak = read_u64le(&self.blob[35..43]) ^ ((&self.state1).into(): &[u64; 25])[24];
        cn_aesni::transplode(
            (&mut self.state0).into(),
            &mut self.memory[..],
            (&self.state1).into(),
        );
        let result = finalize(self.state0);
        self.state0 = self.state1;
        result
    }

    fn mix(&mut self) {
        cn_aesni::mix(&mut self.memory, (&self.state0).into(), self.tweak);
    }
}

impl<Noncer: Iterator<Item = u32>> CryptoNight<Noncer> {
    pub fn new(blob: Vec<u8>, noncer: Noncer) -> Self {
        let mut res = Self {
            memory: Default::default(),
            blob,
            state0: Default::default(),
            state1: Default::default(),
            tweak: Default::default(),
            noncer,
        };
        res.transplode();
        res
    }
}

impl<Noncer: Iterator<Item = u32>> Hasher<Noncer> for CryptoNight<Noncer> {
    fn set_blob(&mut self, blob: Vec<u8>, noncer: Noncer) {
        self.blob = blob;
        self.noncer = noncer;
        self.transplode();
    }

    fn next_hash(&mut self) -> GenericArray<u8, U32> {
        self.mix();
        self.transplode()
    }
}

#[derive(Default)]
pub struct CryptoNight2<Noncer> {
    memory: Mmap<[[i64x2; 1 << 17]; 2]>,
    state: [(State, State); 2],
    tweak: [u64; 2],
    blob: Vec<u8>,
    noncer: Noncer,
    result: Option<GenericArray<u8, U32>>,
}

impl<Noncer: Iterator<Item = u32>> CryptoNight2<Noncer> {
    pub fn new(blob: Vec<u8>, noncer: Noncer) -> Self {
        let mut res = Self {
            memory: Default::default(),
            state: Default::default(),
            tweak: Default::default(),
            blob,
            noncer,
            result: Default::default(),
        };
        res.transplode();
        res
    }

    fn transplode(&mut self) -> [GenericArray<u8, U32>; 2] {
        for (st, tw) in self.state.iter_mut().zip(self.tweak.iter_mut()) {
            set_nonce(&mut self.blob, self.noncer.next().unwrap());
            st.1 = State::from(sha3::Keccak256Full::digest(&self.blob));
            *tw = read_u64le(&self.blob[35..43]) ^ ((&st.1).into(): &[u64; 25])[24];
        }
        for (st, mem) in self.state.iter_mut().zip(self.memory.iter_mut()) {
            cn_aesni::transplode((&mut st.0).into(), &mut mem[..], (&st.1).into());
        }
        let result = [finalize(self.state[0].0), finalize(self.state[1].0)];
        for st in &mut self.state {
            st.0 = st.1;
        }
        result
    }

    fn mix(&mut self) {
        cn_aesni::mix_x2(
            &mut self.memory,
            (&self.state[0].0).into(),
            self.tweak[0],
            (&self.state[1].0).into(),
            self.tweak[1]
        );
    }
}

impl<Noncer: Iterator<Item = u32>> Hasher<Noncer> for CryptoNight2<Noncer> {
    fn set_blob(&mut self, blob: Vec<u8>, noncer: Noncer) {
        self.blob = blob;
        self.noncer = noncer;
        self.transplode();
    }

    fn next_hash(&mut self) -> GenericArray<u8, U32> {
        if let Some(res) = self.result {
            self.result = None;
            return res;
        }
        self.mix();
        let res = self.transplode();
        self.result = Some(res[1]);
        res[0]
    }
}

#[derive(Default)]
pub struct CryptoNightXtl<Noncer> {
    memory: Mmap<[i64x2; 1 << 17]>,
    blob: Vec<u8>,
    state0: State,
    state1: State,
    tweak: u64,
    noncer: Noncer,
}

impl<Noncer: Iterator<Item = u32>> CryptoNightXtl<Noncer> {
    fn transplode(&mut self) -> GenericArray<u8, U32> {
        set_nonce(&mut self.blob, self.noncer.next().unwrap());
        self.state1 = State::from(sha3::Keccak256Full::digest(&self.blob));
        self.tweak = read_u64le(&self.blob[35..43]) ^ ((&self.state1).into(): &[u64; 25])[24];
        cn_aesni::transplode(
            (&mut self.state0).into(),
            &mut self.memory[..],
            (&self.state1).into(),
        );
        let result = finalize(self.state0);
        self.state0 = self.state1;
        result
    }

    fn mix(&mut self) {
        cn_aesni::mix_xtl(&mut self.memory, (&self.state0).into(), self.tweak);
    }
}

impl<Noncer: Iterator<Item = u32>> CryptoNightXtl<Noncer> {
    pub fn new(blob: Vec<u8>, noncer: Noncer) -> Self {
        let mut res = Self {
            memory: Default::default(),
            blob,
            state0: Default::default(),
            state1: Default::default(),
            tweak: Default::default(),
            noncer,
        };
        res.transplode();
        res
    }
}

impl<Noncer: Iterator<Item = u32>> Hasher<Noncer> for CryptoNightXtl<Noncer> {
    fn set_blob(&mut self, blob: Vec<u8>, noncer: Noncer) {
        self.blob = blob;
        self.noncer = noncer;
        self.transplode();
    }

    fn next_hash(&mut self) -> GenericArray<u8, U32> {
        self.mix();
        self.transplode()
    }
}

#[derive(Default)]
pub struct CryptoNightLite<Noncer> {
    memory: Mmap<[i64x2; 1 << 16]>,
    blob: Vec<u8>,
    state0: State,
    state1: State,
    noncer: Noncer,
}

impl<Noncer: Iterator<Item = u32>> CryptoNightLite<Noncer> {
    fn transplode(&mut self) -> GenericArray<u8, U32> {
        set_nonce(&mut self.blob, self.noncer.next().unwrap());
        self.state1 = State::from(sha3::Keccak256Full::digest(&self.blob));
        cn_aesni::transplode(
            (&mut self.state0).into(),
            &mut self.memory[..],
            (&self.state1).into(),
        );
        let result = finalize(self.state0);
        self.state0 = self.state1;
        result
    }

    fn mix(&mut self) {
        cn_aesni::mix_lite(&mut self.memory, (&self.state0).into());
    }
}

impl<Noncer: Iterator<Item = u32>> CryptoNightLite<Noncer> {
    pub fn new(blob: Vec<u8>, noncer: Noncer) -> Self {
        let mut res = Self {
            memory: Default::default(),
            blob,
            state0: Default::default(),
            state1: Default::default(),
            noncer,
        };
        res.transplode();
        res
    }
}

impl<Noncer: Iterator<Item = u32>> Hasher<Noncer> for CryptoNightLite<Noncer> {
    fn set_blob(&mut self, blob: Vec<u8>, noncer: Noncer) {
        self.blob = blob;
        self.noncer = noncer;
        self.transplode();
    }

    fn next_hash(&mut self) -> GenericArray<u8, U32> {
        self.mix();
        self.transplode()
    }
}

#[derive(Default)]
pub struct CryptoNightLite2<Noncer> {
    memory: Mmap<[[i64x2; 1 << 16]; 2]>,
    state: [(State, State); 2],
    blob: Vec<u8>,
    noncer: Noncer,
    result: Option<GenericArray<u8, U32>>,
}

impl<Noncer: Iterator<Item = u32>> CryptoNightLite2<Noncer> {
    pub fn new(blob: Vec<u8>, noncer: Noncer) -> Self {
        let mut res = Self {
            memory: Default::default(),
            state: Default::default(),
            result: Default::default(),
            blob,
            noncer,
        };
        res.transplode();
        res
    }

    fn transplode(&mut self) -> [GenericArray<u8, U32>; 2] {
        for st in &mut self.state {
            set_nonce(&mut self.blob, self.noncer.next().unwrap());
            st.1 = State::from(sha3::Keccak256Full::digest(&self.blob));
        }
        for (st, mem) in self.state.iter_mut().zip(self.memory.iter_mut()) {
            cn_aesni::transplode((&mut st.0).into(), &mut mem[..], (&st.1).into());
        }
        let result = [finalize(self.state[0].0), finalize(self.state[1].0)];
        for st in &mut self.state {
            st.0 = st.1;
        }
        result
    }

    fn mix(&mut self) {
        cn_aesni::mix_lite_x2(
            &mut self.memory,
            (&self.state[0].0).into(),
            (&self.state[1].0).into(),
        );
    }
}

impl<Noncer: Iterator<Item = u32>> Hasher<Noncer> for CryptoNightLite2<Noncer> {
    fn set_blob(&mut self, blob: Vec<u8>, noncer: Noncer) {
        self.blob = blob;
        self.noncer = noncer;
        self.transplode();
    }

    fn next_hash(&mut self) -> GenericArray<u8, U32> {
        if let Some(res) = self.result {
            self.result = None;
            return res;
        }
        self.mix();
        let res = self.transplode();
        self.result = Some(res[1]);
        res[0]
    }
}

#[derive(Default)]
pub struct CryptoNightLiteV1<Noncer> {
    memory: Mmap<[i64x2; 1 << 16]>,
    blob: Vec<u8>,
    state0: State,
    state1: State,
    noncer: Noncer,
}

impl<Noncer: Iterator<Item = u32>> CryptoNightLiteV1<Noncer> {
    fn transplode(&mut self) -> GenericArray<u8, U32> {
        set_nonce(&mut self.blob, self.noncer.next().unwrap());
        self.state1 = State::from(sha3::Keccak256Full::digest(&self.blob));
        cn_aesni::transplode(
            (&mut self.state0).into(),
            &mut self.memory[..],
            (&self.state1).into(),
        );
        let result = finalize(self.state0);
        self.state0 = self.state1;
        result
    }

    fn mix(&mut self) {
        cn_aesni::mix_lite_v1(&mut self.memory, (&self.state0).into());
    }
}

impl<Noncer: Iterator<Item = u32>> CryptoNightLiteV1<Noncer> {
    pub fn new(blob: Vec<u8>, noncer: Noncer) -> Self {
        let mut res = Self {
            memory: Default::default(),
            blob,
            state0: Default::default(),
            state1: Default::default(),
            noncer,
        };
        res.transplode();
        res
    }
}

impl<Noncer: Iterator<Item = u32>> Hasher<Noncer> for CryptoNightLiteV1<Noncer> {
    fn set_blob(&mut self, blob: Vec<u8>, noncer: Noncer) {
        self.blob = blob;
        self.noncer = noncer;
        self.transplode();
    }

    fn next_hash(&mut self) -> GenericArray<u8, U32> {
        self.mix();
        self.transplode()
    }
}

#[derive(Default)]
pub struct CryptoNightHeavy<Noncer> {
    memory: Mmap<[i64x2; 1 << 18]>,
    blob: Vec<u8>,
    state0: State,
    state1: State,
    noncer: Noncer,
}

impl<Noncer: Iterator<Item = u32>> CryptoNightHeavy<Noncer> {
    fn transplode(&mut self) -> GenericArray<u8, U32> {
        set_nonce(&mut self.blob, self.noncer.next().unwrap());
        self.state1 = State::from(sha3::Keccak256Full::digest(&self.blob));
        cn_aesni::transplode_heavy(
            (&mut self.state0).into(),
            &mut self.memory[..],
            (&self.state1).into(),
        );
        let result = finalize(self.state0);
        self.state0 = self.state1;
        result
    }

    fn mix(&mut self) {
        cn_aesni::mix_heavy(&mut self.memory, (&self.state0).into());
    }
}

impl<Noncer: Iterator<Item = u32>> CryptoNightHeavy<Noncer> {
    pub fn new(blob: Vec<u8>, noncer: Noncer) -> Self {
        let mut res = Self {
            memory: Default::default(),
            blob,
            state0: Default::default(),
            state1: Default::default(),
            noncer,
        };
        res.transplode();
        res
    }
}

impl<Noncer: Iterator<Item = u32>> Hasher<Noncer> for CryptoNightHeavy<Noncer> {
    fn set_blob(&mut self, blob: Vec<u8>, noncer: Noncer) {
        self.blob = blob;
        self.noncer = noncer;
        self.transplode();
    }

    fn next_hash(&mut self) -> GenericArray<u8, U32> {
        self.mix();
        self.transplode()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hex_literal::{hex, hex_impl};

    // "official" slow_hash test vectors, from tests-slow-1.txt
    const INPUT0: &[u8] = &hex!(
        "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
    );
    const INPUT1: &[u8] = &hex!("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
    const INPUT2: &[u8] = &hex!("8519e039172b0d70e5ca7b3383d6b3167315a422747b73f019cf9528f0fde341fd0f2a63030ba6450525cf6de31837669af6f1df8131faf50aaab8d3a7405589");
    const INPUT3: &[u8] = &hex!("37a636d7dafdf259b7287eddca2f58099e98619d2f99bdb8969d7b14498102cc065201c8be90bd777323f449848b215d2977c92c4c1c2da36ab46b2e389689ed97c18fec08cd3b03235c5e4c62a37ad88c7b67932495a71090e85dd4020a9300");
    const INPUT4: &[u8] = &hex!(
        "38274c97c45a172cfc97679870422e3a1ab0784960c60514d816271415c306ee3a3ed1a77e31f6a885c3cb"
    );
    const OUTPUT0: &[u8] = &hex!("b5a7f63abb94d07d1a6445c36c07c7e8327fe61b1647e391b4c7edae5de57a3d");
    const OUTPUT1: &[u8] = &hex!("80563c40ed46575a9e44820d93ee095e2851aa22483fd67837118c6cd951ba61");
    const OUTPUT2: &[u8] = &hex!("5bb40c5880cef2f739bdb6aaaf16161eaae55530e7b10d7ea996b751a299e949");
    const OUTPUT3: &[u8] = &hex!("613e638505ba1fd05f428d5c9f8e08f8165614342dac419adc6a47dce257eb3e");
    const OUTPUT4: &[u8] = &hex!("ed082e49dbd5bbe34a3726a0d1dad981146062b39d36d62c71eb1ed8ab49459b");
    const LITEOU0: &[u8] = &hex!("f2360d43b1c6c343e9f53da17e213a51325b05e7909ae9405f828ee45b8282f4");
    const LITEOU1: &[u8] = &hex!("4c3428f39e1f9ecda3b0726fd4f4fca62843597c480f033ae38d113282b273bf");
    const LITEOU2: &[u8] = &hex!("f828ff6bf19a6e72a77319808e43f745f90f47eceb698e4319d2484404b081e8");
    const LITEOU3: &[u8] = &hex!("61e3cb5e046ae4ac5d03f8ec6bd7a9a80d5e2573817429d1624735f66aff4b11");
    const LITEOU4: &[u8] = &hex!("12c2f26b89cc514707ac0fb953f1f72581eb468de2a5d4d4cf95c0d1b32f7285");
    const LIT1OU0: &[u8] = &hex!("5655313715525895d2312bfba9b7f5e45f441b8b8d3957eaea0b6039d1bc0713");
    const LIT1OU1: &[u8] = &hex!("173c9db34c9643ba689e16044f5c273c4c5543b210a4d5248352ac536064a850");
    const LIT1OU2: &[u8] = &hex!("bb196c4c0c9dc1c4e44c2a6f9e61200fe3c8b4ef232134e65c3c7862c7d3df6a");
    const LIT1OU3: &[u8] = &hex!("452103731dd8d70ce32f726b8e71fcd91005fb3cb2abd78f2b7357bb07f8c8bc");
    const LIT1OU4: &[u8] = &hex!("4e785376ed2733262d83cc25321a9d0003f5395315de919acf1b97f0a84fbd2d");
    const HEAVOU0: &[u8] = &hex!("8e9b0d37a75eea071c224d10522b9e12a7f1a96a317efd92db41e593133574a4");
    const HEAVOU1: &[u8] = &hex!("3ec94bf7800410ff50de0767196bc60d90e9598a7c70a7d27f4c090d7f25377a");
    const HEAVOU2: &[u8] = &hex!("f68db02d511e3f6641d770ca907157f2d68e7e08f95fe349ed421e9607eb3d6d");
    const HEAVOU3: &[u8] = &hex!("04b33b61e528836cc825e76ff11967b792bf026129261e05b846e241c286140e");
    const HEAVOU4: &[u8] = &hex!("dfa4fc4da8edc4bbf311e9eacaebfbf91be64061ded4e71d3c9347337a47e7ac");
    const XTLOUT0: &[u8] = &hex!("eee9cd0f335ac687bb68a7155e7bd07be6b639ef133fe8795a47f5d73fa4dff3");
    const XTLOUT1: &[u8] = &hex!("c783fa00e092c9f1c887b18fcb6666f04a0bc3ab59d8262c20febd1248eef4ea");
    const XTLOUT2: &[u8] = &hex!("5b6f54b401ce5546a1426514b40842d09f92411349a5974a323e662c5d9f9b50");
    const XTLOUT3: &[u8] = &hex!("fbc3fc330f1759e118c753daeda4e0914a2d0aaba6be71575b7fcd5175666722");
    const XTLOUT4: &[u8] = &hex!("3bd752507fee87037fe144d84ced691a60b2e7765719385dcac52cbbeaca27ae");

    // test-slow-2.txt
    const NEWIN0: &[u8] = &hex!(
        "5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374");
    const NEWIN1: &[u8] = &hex!(
        "4c6f72656d20697073756d20646f6c6f722073697420616d65742c20636f6e73656374657475722061646970697363696e67");
    const NEWIN2: &[u8] = &hex!(
        "656c69742c2073656420646f20656975736d6f642074656d706f7220696e6369646964756e74207574206c61626f7265");
    const NEWIN3: &[u8] = &hex!(
        "657420646f6c6f7265206d61676e6120616c697175612e20557420656e696d206164206d696e696d2076656e69616d2c");
    const NEWIN4: &[u8] = &hex!(
        "71756973206e6f737472756420657865726369746174696f6e20756c6c616d636f206c61626f726973206e697369");
    const NEWIN5: &[u8] = &hex!(
        "757420616c697175697020657820656120636f6d6d6f646f20636f6e7365717561742e20447569732061757465");
    const NEWIN6: &[u8] = &hex!(
        "697275726520646f6c6f7220696e20726570726568656e646572697420696e20766f6c7570746174652076656c6974");
    const NEWIN7: &[u8] = &hex!(
        "657373652063696c6c756d20646f6c6f726520657520667567696174206e756c6c612070617269617475722e");
    const NEWIN8: &[u8] = &hex!(
        "4578636570746575722073696e74206f6363616563617420637570696461746174206e6f6e2070726f6964656e742c");
    const NEWIN9: &[u8] = &hex!(
        "73756e7420696e2063756c706120717569206f666669636961206465736572756e74206d6f6c6c697420616e696d20696420657374206c61626f72756d2e");

    const CN2OU0: &[u8] = &hex!("2e6ee8cc718c61d3a59ecdfca6e56ca5f560b4bb75c201ed3bb001c407833e79");
    const CN2OU1: &[u8] = &hex!("35957227ce70064db4c1b5ece364282fd5425bf4fee5a0e3595b9f3f5067b90b");
    const CN2OU2: &[u8] = &hex!("f6398c333cb775cbf81fbb1043f0d7c791dac5bf6182f6ad782ba11fe6c7234f");
    const CN2OU3: &[u8] = &hex!("5bb096fc037aabc50d7e84b356ea673357684e7d29395bd32b876440efcbaf72");
    const CN2OU4: &[u8] = &hex!("f3e44470ff4bc947c4cb020168d636fc894c3c07629266b93e2fbcf42d9664a5");
    const CN2OU5: &[u8] = &hex!("13d3dc2794414932050ff7b165de51b283018990f111c3191bca66fe986fdab9");
    const CN2OU6: &[u8] = &hex!("9fcaa9892f1faef36624d865cb7e7588b4b74fc581b7195b586b3b0e802b72b8");
    const CN2OU7: &[u8] = &hex!("30c262cf4592136088dcf1064b732c29550b46accf54d7993d8532f5a5a9e0f3");
    const CN2OU8: &[u8] = &hex!("88536691d2d8eb6c8dfbb2597ab50fbbd9f8c2834281e1bb70616f48094d68c8");
    const CN2OU9: &[u8] = &hex!("5964da99f4a273393e464f40070122f045eecfed1309ac25dd322e1fb052dc45");


    fn test_cn(input: &[u8], output: &[u8], nonce: u32) {
        assert_eq!(
            &((&mut CryptoNight::new(input.iter().cloned().collect(), nonce..) as &mut Hasher<_>)
                .next_hash())[..],
            output
        );
    }

    fn test_cnl(input: &[u8], output: &[u8], nonce: u32) {
        assert_eq!(
            &((&mut CryptoNightLite::new(input.iter().cloned().collect(), nonce..)
                as &mut Hasher<_>)
                .next_hash())[..],
            output
        );
    }

    fn test_cnl_v1(input: &[u8], output: &[u8], nonce: u32) {
        assert_eq!(
            &((&mut CryptoNightLiteV1::new(input.iter().cloned().collect(), nonce..)
                as &mut Hasher<_>)
                .next_hash())[..],
            output
        );
    }

    fn test_cnl_x2(input: &[u8], output: &[u8], nonce: u32) {
        assert_eq!(
            &((&mut CryptoNightLite2::new(input.iter().cloned().collect(), nonce..)
                as &mut Hasher<_>)
                .next_hash())[..],
            output
        );
    }

    fn test_cnh(input: &[u8], output: &[u8], nonce: u32) {
        assert_eq!(
            &((&mut CryptoNightHeavy::new(input.iter().cloned().collect(), nonce..)
                as &mut Hasher<_>)
                .next_hash())[..],
            output
        );
    }

    fn test_xtl(input: &[u8], output: &[u8], nonce: u32) {
        assert_eq!(
            &((&mut CryptoNightXtl::new(input.iter().cloned().collect(), nonce..) as &mut Hasher<_>)
                .next_hash())[..],
            output
        );
    }

    #[test]
    fn test_cn_0() {
        test_cn(INPUT0, OUTPUT0, 0);
    }

    #[test]
    fn test_cn_1() {
        test_cn(INPUT1, OUTPUT1, 0);
    }

    #[test]
    fn test_cn_2() {
        test_cn(INPUT2, OUTPUT2, 0xcf250545);
    }

    #[test]
    fn test_cn_3() {
        test_cn(INPUT3, OUTPUT3, 0xf4237377);
    }

    #[test]
    fn test_cn_4() {
        test_cn(INPUT4, OUTPUT4, 0xcbc385a8);
    }

    #[test]
    fn test_cnl_0() {
        test_cnl(INPUT0, LITEOU0, 0);
    }

    #[test]
    fn test_cnl_1() {
        test_cnl(INPUT1, LITEOU1, 0);
    }

    #[test]
    fn test_cnl_2() {
        test_cnl(INPUT2, LITEOU2, 0xcf250545);
    }

    #[test]
    fn test_cnl_3() {
        test_cnl(INPUT3, LITEOU3, 0xf4237377);
    }

    #[test]
    fn test_cnl_4() {
        test_cnl(INPUT4, LITEOU4, 0xcbc385a8);
    }

    #[test]
    fn test_cnl_x2_0() {
        test_cnl_x2(INPUT0, LITEOU0, 0);
    }

    #[test]
    fn test_cnl_x2_1() {
        test_cnl_x2(INPUT1, LITEOU1, 0);
    }

    #[test]
    fn test_cnl_x2_2() {
        test_cnl_x2(INPUT2, LITEOU2, 0xcf250545);
    }

    #[test]
    fn test_cnl_x2_3() {
        test_cnl_x2(INPUT3, LITEOU3, 0xf4237377);
    }

    #[test]
    fn test_cnl_x2_4() {
        test_cnl_x2(INPUT4, LITEOU4, 0xcbc385a8);
    }

    #[test]
    fn test_cnl_cnl_x2() {
        for (cnl, cnlx2) in (&mut CryptoNightLite::new(INPUT3.iter().cloned().collect(), 0..)
            as &mut Hasher<_>)
            .zip(
                &mut CryptoNightLite2::new(INPUT3.iter().cloned().collect(), 0..) as &mut Hasher<_>,
            )
            .take(5)
        {
            assert_eq!(cnlx2, cnl);
        }
    }

    #[test]
    fn test_cnl_v1_0() {
        test_cnl_v1(INPUT0, LIT1OU0, 0);
    }

    #[test]
    fn test_cnl_v1_1() {
        test_cnl_v1(INPUT1, LIT1OU1, 0);
    }

    #[test]
    fn test_cnl_v1_2() {
        test_cnl_v1(INPUT2, LIT1OU2, 0xcf250545);
    }

    #[test]
    fn test_cnl_v1_3() {
        test_cnl_v1(INPUT3, LIT1OU3, 0xf4237377);
    }

    #[test]
    fn test_cnl_v1_4() {
        test_cnl_v1(INPUT4, LIT1OU4, 0xcbc385a8);
    }

    #[test]
    fn test_cnh_0() {
        test_cnh(INPUT0, HEAVOU0, 0);
    }

    #[test]
    fn test_cnh_1() {
        test_cnh(INPUT1, HEAVOU1, 0);
    }

    #[test]
    fn test_cnh_2() {
        test_cnh(INPUT2, HEAVOU2, 0xcf250545);
    }

    #[test]
    fn test_cnh_3() {
        test_cnh(INPUT3, HEAVOU3, 0xf4237377);
    }

    #[test]
    fn test_cnh_4() {
        test_cnh(INPUT4, HEAVOU4, 0xcbc385a8);
    }

    #[test]
    fn test_xtl_0() {
        test_xtl(INPUT0, XTLOUT0, 0);
    }

    #[test]
    fn test_xtl_1() {
        test_xtl(INPUT1, XTLOUT1, 0);
    }

    #[test]
    fn test_xtl_2() {
        test_xtl(INPUT2, XTLOUT2, 0xcf250545);
    }

    #[test]
    fn test_xtl_3() {
        test_xtl(INPUT3, XTLOUT3, 0xf4237377);
    }

    #[test]
    fn test_xtl_4() {
        test_xtl(INPUT4, XTLOUT4, 0xcbc385a8);
    }
}
