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

pub struct CryptoNight {
    memory: Mmap<[i64x2; 1 << 14]>,
    state0: State,
    state1: State,
    tweak: u64,
}

fn read_u64le(bytes: &[u8]) -> u64 {
    (bytes[0] as u64) | ((bytes[1] as u64) << 8) | ((bytes[2] as u64) << 16)
        | ((bytes[3] as u64) << 24) | ((bytes[4] as u64) << 32) | ((bytes[5] as u64) << 40)
        | ((bytes[6] as u64) << 48) | ((bytes[7] as u64) << 56)
}

impl CryptoNight {
    pub fn new() -> Self {
        CryptoNight {
            memory: Mmap::new_huge().expect("hugepage mmap"),
            state0: State::default(),
            state1: State::default(),
            tweak: u64::default(),
        }
    }

    pub fn init(&mut self, blob: &[u8]) {
        self.state0 = State::from(sha3::Keccak256Full::digest(blob));
        self.tweak = read_u64le(&blob[35..43]) ^ ((&self.state0).into(): &[u64; 25])[24];
        cn_aesni::transplode(
            (&mut self.state1).into(), // dummy buffer, input/output garbage
            &mut self.memory,
            (&self.state0).into(),
        );
    }

    /// "pipelined": returns result for previous input
    pub fn advance(&mut self, blob: &[u8]) -> GenericArray<u8, U32> {
        cn_aesni::mix(&mut self.memory, (&self.state0).into(), self.tweak);
        self.state1 = State::from(sha3::Keccak256Full::digest(blob));
        self.tweak = read_u64le(&blob[35..43]) ^ ((&self.state1).into(): &[u64; 25])[24];
        cn_aesni::transplode(
            (&mut self.state0).into(),
            &mut self.memory,
            (&self.state1).into(),
        );
        let result = finalize(self.state0);
        self.state0 = self.state1;
        result
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
        let mut state = CryptoNight::new().unwrap();
        state.init(&INPUT0[..]);
        let out0 = state.advance(&INPUT1[..]);
        assert_eq!(&out0[..], &OUTPUT0[..]);
    }

    #[test]
    fn test1() {
        let mut state = CryptoNight::new().unwrap();
        state.init(&INPUT0[..]);
        let _ = state.advance(&INPUT1[..]);
        let out1 = state.advance(&INPUT1[..]);
        assert_eq!(&out1[..], &OUTPUT1[..]);
    }

    #[test]
    fn test2() {
        let mut state = CryptoNight::new().unwrap();
        state.init(&INPUT0[..]);
        let _ = state.advance(&INPUT1[..]);
        let _ = state.advance(&INPUT2[..]);
        let out2 = state.advance(&INPUT1[..]);
        assert_eq!(&out2[..], &OUTPUT2[..]);
    }

    #[test]
    fn test3() {
        let mut state = CryptoNight::new().unwrap();
        state.init(&INPUT0[..]);
        let _ = state.advance(&INPUT1[..]);
        let _ = state.advance(&INPUT2[..]);
        let _ = state.advance(&INPUT3[..]);
        let out3 = state.advance(&INPUT1[..]);
        assert_eq!(&out3[..], &OUTPUT3[..]);
    }

    #[test]
    fn test4() {
        let mut state = CryptoNight::new().unwrap();
        state.init(&INPUT0[..]);
        let _ = state.advance(&INPUT1[..]);
        let _ = state.advance(&INPUT2[..]);
        let _ = state.advance(&INPUT3[..]);
        let _ = state.advance(&INPUT4[..]);
        let out4 = state.advance(&INPUT1[..]);
        assert_eq!(&out4[..], &OUTPUT4[..]);
    }
}
