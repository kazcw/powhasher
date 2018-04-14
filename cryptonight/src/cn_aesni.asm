default rel
global cn_mix_v1_x1
global cnl_mix_v0_x1
global cn_transplode

section .text

;mov ebx, 111
;.byte 0x64, 0x67, 0x90

;mov ebx, 222
;.byte 0x64, 0x67, 0x90

%macro defmix 3			; ArenaSz Iters DoTweak
	push   rbx
        pxor   xmm5,xmm5
        pinsrq xmm5,rdx,1
        push   rsi
        movdqa xmm1,[rsi+0x00]
        movdqa xmm2,[rsi+0x10]
        pxor   xmm1,[rsi+0x20]
        pxor   xmm2,[rsi+0x30]
        movq   r8,xmm1
        mov    r10d,%2
        mov    rbx,r8
align 16
.0:
        and    ebx, %1 - 0x10
        movdqa xmm0,[rdi+rbx]	;;
        aesenc xmm0,xmm1	;;
        pxor   xmm2,xmm0
%ifidn %3,cn
        movdqa [rdi+rbx],xmm2
%elifidn %3,cnv1
        movq   rax,xmm2
        mov    [rdi+rbx],rax
        pextrq rsi,xmm2,0x1
        mov    eax,esi
        and    eax,0x31000000
        lea    ecx,[rax+rax*8]
        shr    ecx,26
        and    ecx,0xE
        mov    eax,0x13174000
        shl    eax,cl
        and    eax,0x30000000
        xor    rsi,rax
        mov    [rdi+rbx+8],rsi
%else
%error "unknown variant"
%endif

        movq   rsi,xmm0		;;
        mov    rax,rsi
        and    esi,%1 - 0x10
        mov    r9,[rdi+rsi]	;;
        mul    r9		;;
        lea    ebx,[r8+rdx]
        xor    ebx,r9d
        add    r8d,edx
        xor    r8d,r9d
%ifidn %3,cn
        movdqa xmm4,[rdi+rsi]
%elifidn %3,cnv1
        movdqa xmm4,xmm5
        pxor   xmm4,[rdi+rsi]
%else
%error "unknown variant"
%endif
        movq   xmm3,rdx		;;
        pinsrq xmm3,rax,0x1	;;
        paddq  xmm1,xmm3	;;
%ifidn %3,cnv1
        pxor   xmm1,xmm5	;;
%endif
        movdqa [rdi+rsi],xmm1	;;
        pxor   xmm1,xmm4	;;
        dec    r10d
        movdqa xmm2,xmm0
        jne .0
        pop    rsi
	pop    rbx
	ret
%endmacro

cn_mix_v1_x1: defmix 0x200000, 0x80000, cnv1
cnl_mix_v0_x1: defmix 0x100000, 0x40000, cn

cn_transplode:
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
	push   rbx
	push   rcx
	push   rdx
align 16
.0:
	pxor   xmm0,[rdx]
	pxor   xmm1,[rdx+0x10]
	pxor   xmm2,[rdx+0x20]
	pxor   xmm3,[rdx+0x30]
	pxor   xmm4,[rdx+0x40]
	pxor   xmm5,[rdx+0x50]
	pxor   xmm6,[rdx+0x60]
	pxor   xmm7,[rdx+0x70]
	xor    eax,eax
.1:
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
	jne .1
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
	jne .0
	pop    rdx
	pop    rcx
	pop    rbx
	movntdq [rcx+0x00],xmm0
	movntdq [rcx+0x10],xmm1
	movntdq [rcx+0x20],xmm2
	movntdq [rcx+0x30],xmm3
	movntdq [rcx+0x40],xmm4
	movntdq [rcx+0x50],xmm5
	movntdq [rcx+0x60],xmm6
	movntdq [rcx+0x70],xmm7
	ret
