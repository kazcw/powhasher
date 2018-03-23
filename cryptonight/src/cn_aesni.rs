// copyright 2017 Kaz Wesley

use aesni;
use stdsimd::simd::i64x2;

pub fn mix(memory: &mut [i64x2; 1 << 14], from: &[i64x2]) {
    unsafe {
        asm!("
        push   rsi
        movdqa xmm1,[rsi+0x00]
        movdqa xmm2,[rsi+0x10]
        pxor   xmm1,[rsi+0x20]
        pxor   xmm2,[rsi+0x30]
        movq   r8,xmm1
        mov    ecx,0x80000
        mov    rbx,r8
    .align 16
    ${:private}cnmix0${:uid}:
        and    ebx,0x1ffff0
        movdqa xmm0,[rdi+rbx]
        aesenc xmm0,xmm1
        movq   rsi,xmm0
        mov    rax,rsi
        pxor   xmm2,xmm0
        and    esi,0x1ffff0
        movdqa [rdi+rbx],xmm2
        mov    r9,[rdi+rsi]
        mul    r9
        add    r8,rdx
        xor    r8,r9
        mov    rbx,r8
        movq   xmm3,rdx
        movdqa xmm4,[rdi+rsi]
        pinsrq xmm3,rax,0x1
        paddq  xmm1,xmm3
        movdqa [rdi+rsi],xmm1
        pxor   xmm1,xmm4
        dec    ecx
        movdqa xmm2,xmm0
        jne ${:private}cnmix0${:uid}
        pop    rsi
    "::"{rdi}"(memory), "{rsi}"(from.as_ptr())
             :"cc","memory",
             "ecx", "r8", "r9", "rax", "rbx", "xmm0", "xmm1", "xmm2", "xmm3", "xmm4"
             :"intel");
    }
}

pub fn transplode(into: &mut [i64x2], memory: &mut [i64x2; 1 << 14], from: &[i64x2]) {
    let key_into = aesni::genkey(&into[2..4]);
    let key_from = aesni::genkey(&from[0..2]);
    unsafe {
        asm!("
    movdqa xmm0,[rcx+0x00]
    movdqa xmm1,[rcx+0x10]
    movdqa xmm2,[rcx+0x20]
    movdqa xmm3,[rcx+0x30]
    movdqa xmm4,[rcx+0x40]
    movdqa xmm5,[rcx+0x50]
    movdqa xmm6,[rcx+0x60]
    movdqa xmm7,[rcx+0x70]
    movdqa xmm8,[r8+0x00]
    movdqa xmm9,[r8+0x10]
    movdqa xmm10,[r8+0x20]
    movdqa xmm11,[r8+0x30]
    movdqa xmm12,[r8+0x40]
    movdqa xmm13,[r8+0x50]
    movdqa xmm14,[r8+0x60]
    movdqa xmm15,[r8+0x70]
    push   rcx
    push   rdx
    lea    r9,[rdx+0x200000]
${:private}cnsplode0${:uid}:
    pxor   xmm0,[rdx]
    pxor   xmm1,[rdx+0x10]
    pxor   xmm2,[rdx+0x20]
    pxor   xmm3,[rdx+0x30]
    pxor   xmm4,[rdx+0x40]
    pxor   xmm5,[rdx+0x50]
    pxor   xmm6,[rdx+0x60]
    pxor   xmm7,[rdx+0x70]
    xor    eax,eax
${:private}cnsplode1${:uid}:
    lea    rbx,[rdi+rax]
    lea    rcx,[rsi+rax]
    aesenc xmm0,[rbx]
    aesenc xmm8,[rcx]
    aesenc xmm1,[rbx]
    aesenc xmm9,[rcx]
    aesenc xmm2,[rbx]
    aesenc xmm10,[rcx]
    aesenc xmm3,[rbx]
    aesenc xmm11,[rcx]
    aesenc xmm4,[rbx]
    aesenc xmm12,[rcx]
    aesenc xmm5,[rbx]
    aesenc xmm13,[rcx]
    aesenc xmm6,[rbx]
    aesenc xmm14,[rcx]
    aesenc xmm7,[rbx]
    aesenc xmm15,[rcx]
    add    eax,0x10
    cmp    eax,0xa0
    jne ${:private}cnsplode1${:uid}
    movdqa [rdx+0x00],xmm8
    movdqa [rdx+0x10],xmm9
    movdqa [rdx+0x20],xmm10
    movdqa [rdx+0x30],xmm11
    movdqa [rdx+0x40],xmm12
    movdqa [rdx+0x50],xmm13
    movdqa [rdx+0x60],xmm14
    movdqa [rdx+0x70],xmm15
    add    rdx,0x80
    cmp    r9,rdx
    jne ${:private}cnsplode0${:uid}
    pop    rdx
    pop    rcx
    movntdq [rcx+0x00],xmm0
    movntdq [rcx+0x10],xmm1
    movntdq [rcx+0x20],xmm2
    movntdq [rcx+0x30],xmm3
    movntdq [rcx+0x40],xmm4
    movntdq [rcx+0x50],xmm5
    movntdq [rcx+0x60],xmm6
    movntdq [rcx+0x70],xmm7
"
             :
             :"{rdi}"(key_into[..].as_ptr())
             ,"{rsi}"(key_from[..].as_ptr())
             ,"{rdx}"(memory)
             ,"{rcx}"(into[4..].as_mut_ptr())
             ,"{r8}"(from[4..].as_ptr())
             :"cc","memory"
             ,"xmm0","xmm1","xmm2","xmm3","xmm4","xmm5","xmm6","xmm7"
             ,"xmm8","xmm9","xmm10","xmm11","xmm12","xmm13","xmm14","xmm15"
             ,"r9","rax","rbx"
             :"intel");
    }
}
