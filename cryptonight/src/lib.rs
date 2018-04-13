// copyright 2017 Kaz Wesley

#![feature(asm)]
#![feature(attr_literals)]
#![feature(ptr_internals)]
#![feature(repr_simd)]
#![feature(stdsimd)]
#![feature(type_ascription)]
#![feature(unique)]
#![feature(untagged_unions)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(hex_literals))]

extern crate blake;
extern crate groestl_aesni;
extern crate jh_x86_64;
extern crate keccak;
extern crate libc;
extern crate sha3;
extern crate skein;

mod aesni;
mod cn_aesni;
mod mmap;
mod state;

use blake::digest::Digest;
use mmap::Mmap;
use skein::digest::generic_array::typenum::U32;
use skein::digest::generic_array::GenericArray;
use state::State;
use std::simd::i64x2;

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
    (bytes[0] as u64) | ((bytes[1] as u64) << 8) | ((bytes[2] as u64) << 16)
        | ((bytes[3] as u64) << 24) | ((bytes[4] as u64) << 32) | ((bytes[5] as u64) << 40)
        | ((bytes[6] as u64) << 48) | ((bytes[7] as u64) << 56)
}

fn set_nonce(blob: &mut [u8], nonce: u32) {
    blob[39] = (nonce >> 0x18) as u8;
    blob[40] = (nonce >> 0x10) as u8;
    blob[41] = (nonce >> 0x08) as u8;
    blob[42] = (nonce >> 0x00) as u8;
}

pub struct CryptoNight {
    memory: Mmap<[i64x2; 1 << 14]>,
    blob: Vec<u8>,
}

pub struct CryptoNightIterator<'a, T> {
    memory: &'a mut [i64x2; 1 << 14],
    blob: &'a mut [u8],
    state0: State,
    state1: State,
    tweak: u64,
    noncer: T,
}

impl<'a, T: Iterator<Item = u32>> CryptoNightIterator<'a, T> {
    pub fn new(
        memory: &'a mut Mmap<[i64x2; 1 << 14]>,
        blob: &'a mut Vec<u8>,
        mut noncer: T,
    ) -> Self {
        set_nonce(blob, noncer.next().unwrap());
        let state0 = State::from(sha3::Keccak256Full::digest(&blob));
        let mut state1 = State::default();
        cn_aesni::transplode((&mut state1).into(), memory, (&state0).into());
        let tweak = read_u64le(&blob[35..43]) ^ ((&state0).into(): &[u64; 25])[24];
        CryptoNightIterator {
            memory,
            blob,
            state0,
            state1,
            tweak,
            noncer,
        }
    }
}

impl<'a, T: Iterator<Item = u32>> Iterator for CryptoNightIterator<'a, T> {
    type Item = GenericArray<u8, U32>;

    fn next(&mut self) -> Option<Self::Item> {
        set_nonce(self.blob, self.noncer.next().unwrap());
        cn_aesni::mix(self.memory, (&self.state0).into(), self.tweak);
        self.state1 = State::from(sha3::Keccak256Full::digest(&self.blob));
        self.tweak = read_u64le(&self.blob[35..43]) ^ ((&self.state1).into(): &[u64; 25])[24];
        cn_aesni::transplode(
            (&mut self.state0).into(),
            self.memory,
            (&self.state1).into(),
        );
        let result = Some(finalize(self.state0));
        self.state0 = self.state1;
        result
    }
}

impl CryptoNight {
    pub fn new() -> Self {
        CryptoNight {
            memory: Mmap::new_huge().expect("hugepage mmap"),
            blob: Default::default(),
        }
    }

    pub fn hashes<T: Iterator<Item = u32>>(
        &mut self,
        blob: Vec<u8>,
        noncer: T,
    ) -> CryptoNightIterator<T> {
        self.blob = blob;
        CryptoNightIterator::new(&mut self.memory, &mut self.blob, noncer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // "official" slow_hash test vectors, from tests-slow-1.txt
    const INPUT0: &[u8] = hex!(
        "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
    );
    const INPUT1: &[u8] = hex!("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
    const INPUT2: &[u8] = hex!("8519e039172b0d70e5ca7b3383d6b3167315a422747b73f019cf9528f0fde341fd0f2a63030ba6450525cf6de31837669af6f1df8131faf50aaab8d3a7405589");
    const INPUT3: &[u8] = hex!("37a636d7dafdf259b7287eddca2f58099e98619d2f99bdb8969d7b14498102cc065201c8be90bd777323f449848b215d2977c92c4c1c2da36ab46b2e389689ed97c18fec08cd3b03235c5e4c62a37ad88c7b67932495a71090e85dd4020a9300");
    const INPUT4: &[u8] = hex!(
        "38274c97c45a172cfc97679870422e3a1ab0784960c60514d816271415c306ee3a3ed1a77e31f6a885c3cb"
    );
    const OUTPUT0: &[u8] = hex!("b5a7f63abb94d07d1a6445c36c07c7e8327fe61b1647e391b4c7edae5de57a3d");
    const OUTPUT1: &[u8] = hex!("80563c40ed46575a9e44820d93ee095e2851aa22483fd67837118c6cd951ba61");
    const OUTPUT2: &[u8] = hex!("5bb40c5880cef2f739bdb6aaaf16161eaae55530e7b10d7ea996b751a299e949");
    const OUTPUT3: &[u8] = hex!("613e638505ba1fd05f428d5c9f8e08f8165614342dac419adc6a47dce257eb3e");
    const OUTPUT4: &[u8] = hex!("ed082e49dbd5bbe34a3726a0d1dad981146062b39d36d62c71eb1ed8ab49459b");

    #[test]
    fn test0() {
        let out0 = CryptoNight::new()
            .hashes(INPUT0.iter().cloned().collect(), 0..)
            .next()
            .unwrap();
        assert_eq!(&out0[..], &OUTPUT0[..]);
    }

    #[test]
    fn test1() {
        let out1 = CryptoNight::new()
            .hashes(INPUT1.iter().cloned().collect(), 0..)
            .next()
            .unwrap();
        assert_eq!(&out1[..], &OUTPUT1[..]);
    }

    #[test]
    fn test2() {
        let out2 = CryptoNight::new()
            .hashes(INPUT2.iter().cloned().collect(), 0x450525cf..)
            .next()
            .unwrap();
        assert_eq!(&out2[..], &OUTPUT2[..]);
    }

    #[test]
    fn test3() {
        let out3 = CryptoNight::new()
            .hashes(INPUT3.iter().cloned().collect(), 0x777323f4..)
            .next()
            .unwrap();
        assert_eq!(&out3[..], &OUTPUT3[..]);
    }

    #[test]
    fn test4() {
        let out4 = CryptoNight::new()
            .hashes(INPUT4.iter().cloned().collect(), 0xa885c3cb..)
            .next()
            .unwrap();
        assert_eq!(&out4[..], &OUTPUT4[..]);
    }
}
