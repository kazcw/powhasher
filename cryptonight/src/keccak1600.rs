use std::mem;
use tiny_keccak::Keccak;

/// Use for extracting raw state. Very evil.
/// Must pin exact `tiny_keccak` version since we're counting on implementation details.
struct KeccakGuts {
    a: [u64; 25],
    _offset: usize,
    _rate: usize,
    _delim: u8,
}

/// Hash function as used by `CryptoNight`. Equivalent to doing a
/// standard SHA3 with rate=136 and then dumping the whole 1600-bit
/// state rather than sponging it out.
pub fn keccak1600(data: &[u8]) -> [u64; 25] {
    let mut hasher = Keccak::new(136, 0x01);
    hasher.update(data);
    hasher.pad();
    hasher.keccakf();
    (unsafe { mem::transmute(hasher) }: KeccakGuts).a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keccak1600_bytes(data: &[u8]) -> [u8; 200] {
        let hash = keccak1600(data);
        unsafe { mem::transmute(hash) }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    const INPUT1: &[u8] = &[0xcc];
    #[cfg_attr(rustfmt, rustfmt_skip)]
    const OUTPUT0: &[u8] = &[0xc5,0xd2,0x46,0x01,0x86,0xf7,0x23,0x3c,0x92,0x7e,0x7d,0xb2,0xdc,0xc7,
        0x03,0xc0,0xe5,0x00,0xb6,0x53,0xca,0x82,0x27,0x3b,0x7b,0xfa,0xd8,0x04,0x5d,0x85,0xa4,0x70];
    #[cfg_attr(rustfmt, rustfmt_skip)]
    const OUTPUT1: &[u8] = &[0xee,0xad,0x6d,0xbf,0xc7,0x34,0x0a,0x56,0xca,0xed,0xc0,0x44,0x69,0x6a,
        0x16,0x88,0x70,0x54,0x9a,0x6a,0x7f,0x6f,0x56,0x96,0x1e,0x84,0xa5,0x4b,0xd9,0x97,0x0b,0x8a];

    #[test]
    fn test0() {
        let out0 = keccak1600_bytes(&[]);
        assert_eq!(&out0[..32], &OUTPUT0[..]);
    }

    #[test]
    fn test1() {
        let out1 = keccak1600_bytes(&INPUT1[..]);
        assert_eq!(&out1[..32], &OUTPUT1[..]);
    }
}
