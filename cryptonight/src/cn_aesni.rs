// copyright 2017 Kaz Wesley

use crate::aesni;
use std::os::raw::c_void;
use std::arch::x86_64::__m128i as i64x2;

#[link(name = "cnaesni")]
extern "C" {
    pub fn cn_mix_v1_x1(memory: *mut c_void, from: *const c_void, tweak: u64);
    pub fn cn_mix_v1xtl_x1(memory: *mut c_void, from: *const c_void, tweak: u64);
    pub fn cn_mix_v1_x2(memory: *mut c_void, from0: *const c_void, from1: *const c_void, tweak0: u64, tweak1: u64);
    pub fn cnl_mix_v0_x1(memory: *mut c_void, from: *const c_void);
    pub fn cnl_mix_v0_x2(memory: *mut c_void, from0: *const c_void, from1: *const c_void);
    pub fn cnl_mix_v1_x1(memory: *mut c_void, from: *const c_void);
    pub fn cnh_mix(memory: *mut c_void, from: *const c_void);
    pub fn cn_transplode(
        key_into: *const c_void,
        key_from: *const c_void,
        memory: *mut c_void,
        into: *mut c_void,
        from: *const c_void,
        mem_end: *mut c_void,
    );
    pub fn cnh_transplode(
        key_into: *const c_void,
        key_from: *const c_void,
        memory: *mut c_void,
        into: *mut c_void,
        from: *const c_void,
        mem_end: *mut c_void,
    );
}

pub fn mix(memory: &mut [i64x2; 1 << 17], from: &[i64x2], tweak: u64) {
    unsafe {
        cn_mix_v1_x1(
            memory.as_mut_ptr() as *mut c_void,
            from.as_ptr() as *const c_void,
            tweak,
        );
    }
}

pub fn mix_xtl(memory: &mut [i64x2; 1 << 17], from: &[i64x2], tweak: u64) {
    unsafe {
        cn_mix_v1xtl_x1(
            memory.as_mut_ptr() as *mut c_void,
            from.as_ptr() as *const c_void,
            tweak,
        );
    }
}

pub fn mix_x2(memory: &mut [[i64x2; 1 << 17]; 2], from0: &[i64x2], tweak0: u64, from1: &[i64x2], tweak1: u64) {
    unsafe {
        cn_mix_v1_x2(
            memory.as_mut_ptr() as *mut c_void,
            from0.as_ptr() as *const c_void,
            from1.as_ptr() as *const c_void,
            tweak0,
            tweak1,
        );
    }
}

pub fn mix_lite(memory: &mut [i64x2; 1 << 16], from: &[i64x2]) {
    unsafe {
        cnl_mix_v0_x1(
            memory.as_mut_ptr() as *mut c_void,
            from.as_ptr() as *const c_void,
        );
    }
}

pub fn mix_lite_v1(memory: &mut [i64x2; 1 << 16], from: &[i64x2]) {
    unsafe {
        cnl_mix_v1_x1(
            memory.as_mut_ptr() as *mut c_void,
            from.as_ptr() as *const c_void,
        );
    }
}

pub fn mix_heavy(memory: &mut [i64x2; 1 << 18], from: &[i64x2]) {
    unsafe {
        cnh_mix(
            memory.as_mut_ptr() as *mut c_void,
            from.as_ptr() as *const c_void,
        );
    }
}

pub fn mix_lite_x2(memory: &mut [[i64x2; 1 << 16]; 2], from0: &[i64x2], from1: &[i64x2]) {
    unsafe {
        cnl_mix_v0_x2(
            memory.as_mut_ptr() as *mut c_void,
            from0.as_ptr() as *const c_void,
            from1.as_ptr() as *const c_void,
        );
    }
}

pub fn transplode(into: &mut [i64x2], memory: &mut [i64x2], from: &[i64x2]) {
    let key_into = aesni::genkey(&into[2..4]);
    let key_from = aesni::genkey(&from[0..2]);
    unsafe {
        cn_transplode(
            key_into[..].as_ptr() as *const c_void,
            key_from[..].as_ptr() as *const c_void,
            memory.as_mut_ptr() as *mut c_void,
            into[4..].as_mut_ptr() as *mut c_void,
            from[4..].as_ptr() as *const c_void,
            memory.as_mut_ptr().add(memory.len()) as *mut c_void,
        );
    }
}

pub fn transplode_heavy(into: &mut [i64x2], memory: &mut [i64x2], from: &[i64x2]) {
    let key_into = aesni::genkey(&into[2..4]);
    let key_from = aesni::genkey(&from[0..2]);
    unsafe {
        cnh_transplode(
            key_into[..].as_ptr() as *const c_void,
            key_from[..].as_ptr() as *const c_void,
            memory.as_mut_ptr() as *mut c_void,
            into[4..].as_mut_ptr() as *mut c_void,
            from[4..].as_ptr() as *const c_void,
            memory.as_mut_ptr().add(memory.len()) as *mut c_void,
        );
    }
}
