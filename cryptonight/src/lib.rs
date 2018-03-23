// copyright 2017 Kaz Wesley

#![feature(asm)]
#![feature(attr_literals)]
#![feature(ptr_internals)]
#![feature(repr_simd)]
#![feature(type_ascription)]
#![feature(unique)]
#![feature(untagged_unions)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(hex_literals))]

extern crate blake;
extern crate groestl_aesni;
extern crate jh_x86_64;
extern crate libc;
extern crate skein;
extern crate stdsimd;
extern crate tiny_keccak;

mod aesni;
mod cn_aesni;
mod keccak1600;
mod mmap;
mod state;

use blake::digest::Digest;
use mmap::Mmap;
use simdty::i64x2;
use skein::digest::generic_array::typenum::U32;
use skein::digest::generic_array::GenericArray;
use state::State;
use stdsimd::simd::i64x2;

fn finalize(mut data: State) -> GenericArray<u8, U32> {
    tiny_keccak::keccakf((&mut data).into());
    let bytes: &[u8; 200] = (&data).into();
    match bytes[0] & 3 {
        0 => blake::Blake256::digest(bytes),
        1 => groestl_aesni::Groestl256::digest(bytes),
        2 => jh_x86_64::Jh256::digest(bytes),
        3 => skein::Skein512::<U32>::digest(bytes),
        _ => unreachable!(),
    }
}

pub struct Hashstate {
    memory: Mmap<[i64x2; 1 << 14]>,
    state0: State,
    state1: State,
}

impl Hashstate {
    pub fn new() -> Result<Self, ()> {
        Ok(Hashstate {
            memory: Mmap::new_huge().expect("hugepage mmap"),
            state0: State::default(),
            state1: State::default(),
        })
    }

    pub fn init(&mut self, blob: &[u8]) {
        self.state0 = State::from(keccak1600::keccak1600(blob));
        cn_aesni::transplode(
            (&mut self.state1).into(), // dummy buffer, input/output garbage
            &mut self.memory,
            (&self.state0).into(),
        );
    }

    /// "pipelined": returns result for previous input
    pub fn advance(&mut self, blob: &[u8]) -> GenericArray<u8, U32> {
        cn_aesni::mix(&mut self.memory, (&self.state0).into());
        self.state1 = State::from(keccak1600::keccak1600(blob));
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

    // "official" slow_hash test vectors
    const INPUT0: &[u8] = hex!("6465206f6d6e69627573206475626974616e64756d");
    const INPUT1: &[u8] = hex!("6162756e64616e732063617574656c61206e6f6e206e6f636574");
    const INPUT2: &[u8] = hex!("63617665617420656d70746f72");
    const INPUT3: &[u8] = hex!("6578206e6968696c6f206e6968696c20666974");
    const OUTPUT0: &[u8] = hex!("2f8e3df40bd11f9ac90c743ca8e32bb391da4fb98612aa3b6cdc639ee00b31f5");
    const OUTPUT1: &[u8] = hex!("722fa8ccd594d40e4a41f3822734304c8d5eff7e1b528408e2229da38ba553c4");
    const OUTPUT2: &[u8] = hex!("bbec2cacf69866a8e740380fe7b818fc78f8571221742d729d9d02d7f8989b87");
    const OUTPUT3: &[u8] = hex!("b1257de4efc5ce28c6b40ceb1c6c8f812a64634eb3e81c5220bee9b2b76a6f05");

    #[test]
    fn test0() {
        let mut state = Hashstate::new().unwrap();
        state.init(&INPUT0[..]);
        let out0 = state.advance(&INPUT1[..]);
        assert_eq!(&out0[..], &OUTPUT0[..]);
    }

    #[test]
    fn test1() {
        let mut state = Hashstate::new().unwrap();
        state.init(&INPUT0[..]);
        let _ = state.advance(&INPUT1[..]);
        let out1 = state.advance(&INPUT2[..]);
        assert_eq!(&out1[..], &OUTPUT1[..]);
    }

    #[test]
    fn test2() {
        let mut state = Hashstate::new().unwrap();
        state.init(&INPUT0[..]);
        let _ = state.advance(&INPUT1[..]);
        let _ = state.advance(&INPUT2[..]);
        let out2 = state.advance(&INPUT3[..]);
        assert_eq!(&out2[..], &OUTPUT2[..]);
    }

    #[test]
    fn test3() {
        let mut state = Hashstate::new().unwrap();
        state.init(&INPUT0[..]);
        let _ = state.advance(&INPUT1[..]);
        let _ = state.advance(&INPUT2[..]);
        let _ = state.advance(&INPUT3[..]);
        let out3 = state.advance(&[]);
        assert_eq!(&out3[..], &OUTPUT3[..]);
    }
}
