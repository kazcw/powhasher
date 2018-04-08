// copyright 2017 Kaz Wesley

//! 200-byte buffer with 1/8/16-byte views.

use stdsimd::simd::i64x2;

#[derive(Clone, Copy)]
#[repr(C, align(128))]
pub union State {
    // full-size (array interface)
    u8_array: [u8; 200],
    u64_array: [u64; 25],
    // partial!
    i64x2_array: [i64x2; 12],
}

impl Default for State {
    fn default() -> Self {
        State {
            u64_array: [0u64; 25],
        }
    }
}

impl From<[u64; 25]> for State {
    fn from(u64_array: [u64; 25]) -> State {
        State { u64_array }
    }
}

impl<'a> From<&'a State> for &'a [u8; 200] {
    fn from(state: &'a State) -> Self {
        unsafe { &state.u8_array }
    }
}

impl<'a> From<&'a mut State> for &'a mut [u64; 25] {
    fn from(state: &'a mut State) -> Self {
        unsafe { &mut state.u64_array }
    }
}

impl<'a> From<&'a State> for &'a [u64; 25] {
    fn from(state: &'a State) -> Self {
        unsafe { &state.u64_array }
    }
}

impl<'a> From<&'a State> for &'a [i64x2] {
    fn from(state: &'a State) -> Self {
        unsafe { &state.i64x2_array[..] }
    }
}

impl<'a> From<&'a mut State> for &'a mut [i64x2] {
    fn from(state: &'a mut State) -> Self {
        unsafe { &mut state.i64x2_array[..] }
    }
}
