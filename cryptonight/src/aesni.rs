// copyright 2017 Kaz Wesley

use stdsimd::simd::i64x2;

macro_rules! expand_round {
    ($round:expr, $mask:expr, $in0:ident, $in1:ident) => {{
        let output: i64x2;
        asm!(concat!("
            aeskeygenassist xmm2, xmm1, ", $round,
                                                    "
            pshufd xmm2, xmm2, ", $mask,
                                                    "
            movdqa xmm3, xmm0
            pslldq xmm3, 0x4
            pxor   xmm0, xmm3
            pslldq xmm3, 0x4
            pxor   xmm0, xmm3
            pslldq xmm3, 0x4
            pxor   xmm0, xmm3
            pxor   xmm0, xmm2
            ")
                                                    : "={xmm0}"(output)
                                                    : "{xmm0}"($in0),"{xmm1}"($in1)
                                                    : "xmm1", "xmm2", "xmm3"
                                                    : "intel"
                                                );
        output
    }};
}

pub fn genkey(inputs: &[i64x2]) -> [i64x2; 10] {
    let k0 = inputs[0];
    let k1 = inputs[1];
    debug_assert!(inputs.len() == 2);
    unsafe {
        let k2 = expand_round!("0x01", "0xFF", k0, k1);
        let k3 = expand_round!("0x00", "0xAA", k1, k2);
        let k4 = expand_round!("0x02", "0xFF", k2, k3);
        let k5 = expand_round!("0x00", "0xAA", k3, k4);
        let k6 = expand_round!("0x04", "0xFF", k4, k5);
        let k7 = expand_round!("0x00", "0xAA", k5, k6);
        let k8 = expand_round!("0x08", "0xFF", k6, k7);
        let k9 = expand_round!("0x00", "0xAA", k7, k8);
        [k0, k1, k2, k3, k4, k5, k6, k7, k8, k9]
    }
}
