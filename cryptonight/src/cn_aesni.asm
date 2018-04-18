default rel
global cn_mix_v1_x1
global cnl_mix_v0_x1
global cnl_mix_v0_x2
global cnh_mix
global cn_transplode
global cnh_transplode

section .text

%macro iaca_start 0
	mov ebx, 111
	db 0x64, 0x67, 0x90
%endmacro

%macro iaca_end 0
	mov ebx, 222
	db 0x64, 0x67, 0x90
%endmacro

%macro defmix 3			; ArenaSz Iters DoTweak
	push   rbp
	push   rbx
        pxor   xmm5,xmm5
        pinsrq xmm5,rdx,1
        push   rsi
        movaps xmm1,[rsi+0x00]
        movaps xmm2,[rsi+0x10]
        pxor   xmm1,[rsi+0x20]
        pxor   xmm2,[rsi+0x30]
        movq   r8,xmm1
        mov    r10d,%2
        mov    rbx,r8
align 16
.0:
        and    ebx, %1 - 0x10   ;; 
        movaps xmm0,[rdi+rbx]	;;
        aesenc xmm0,xmm1	;;
        pxor   xmm2,xmm0
%ifidn %3,cnv1
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
        movaps [rdi+rbx],xmm2
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
%ifidn %3,cnv1
        movdqa xmm4,xmm5
        pxor   xmm4,[rdi+rsi]
%else
        movaps xmm4,[rdi+rsi]
%endif
        movq   xmm3,rdx		;;
        pinsrq xmm3,rax,0x1	;;
%ifidn %3,cnhx
	or     eax,5
	mov    ebp,eax
	mov    rax,rdx
	xor    edx,edx
	idiv   rbp

	shr    ebx,2
        and    ebx, (%1 - 0x10)/4
	mov    ebx,[rdi+rbx+%1]
%endif
        paddq  xmm1,xmm3	;;
%ifidn %3,cnv1
        pxor   xmm1,xmm5	;;
%endif
        movaps [rdi+rsi],xmm1	;;
        pxor   xmm1,xmm4
%ifidn %3,cnh
        and    ebx, %1 - 0x10	;; 
	xor    edx,edx
	mov    rax,[rdi+rbx]
	mov    esi,[rdi+rbx+8]	;; 
	lea    rbp,[rdi+rbx]
	mov    ebx,esi
	or     esi,5		;; 
	idiv   rsi		;; 
	xor    [rbp],rax
	xor    ebx,eax		;; 
%endif
        dec    r10d
        movdqa xmm2,xmm0
        jne .0
        pop    rsi
	pop    rbx
	pop    rbp
	ret
%endmacro

%macro defmix2 3		; ArenaSz Iters DoTweak
	push   rbx
	push   r12
	push   r13
	push   r14
	push   r15
%ifidn %3,cnv1
        pxor   xmm5,xmm5
        pinsrq xmm5,rdx,1
%endif
        push   rsi
        movaps xmm1,[rsi+0x00]
        movaps xmm2,[rsi+0x10]
        pxor   xmm1,[rsi+0x20]
        pxor   xmm2,[rsi+0x30]
        movaps xmm9,[rdx+0x00]
        movaps xmm10,[rdx+0x10]
        pxor   xmm9,[rdx+0x20]
        pxor   xmm10,[rdx+0x30]
        mov    r10d,%2
        movq   r8,xmm1
        mov    ebx,r8d
        movq   r14,xmm9
        mov    ecx,r14d
align 16
.0:
        and    ebx, %1 - 0x10
        movaps xmm0,[rdi+rbx]	;;
        aesenc xmm0,xmm1	;;
        pxor   xmm2,xmm0

        and    ecx, %1 - 0x10
        movaps xmm8,[rdi+rcx+%1]	;;
        aesenc xmm8,xmm9	;;
        pxor   xmm10,xmm8
%ifidn %3,cnv1
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
        movaps [rdi+rbx],xmm2
        movaps [rdi+rcx+%1],xmm10
%endif
        movq   rsi,xmm0		;;
        mov    rax,rsi
        and    esi,%1 - 0x10
        mov    r9,[rdi+rsi]	;;

	movq   r11,xmm8
	mov    r12,r11
	and    r11d,%1 - 0x10
	mov    r13,[rdi+r11+%1]

        mul    r9		;;
        lea    ebx,[r8+rdx]
        xor    ebx,r9d
        add    r8d,edx
        xor    r8d,r9d
%ifidn %3,cnv1
        movdqa xmm4,xmm5
        pxor   xmm4,[rdi+rsi]
%else
        movaps xmm4,[rdi+rsi]
%endif
        movq   xmm3,rdx		;;
        pinsrq xmm3,rax,0x1	;;

	mov    rax,r12
        mul    r13		;;
        lea    ecx,[r14+rdx+%1]
        xor    ecx,r13d
        add    r14d,edx
        xor    r14d,r13d
%ifidn %3,cnv1
        movdqa xmm4,xmm5
        pxor   xmm4,[rdi+rsi]
%else
        movaps xmm12,[rdi+r11+%1]
%endif
        movq   xmm11,rdx	;;
        pinsrq xmm11,rax,0x1	;;

        paddq  xmm1,xmm3	;;
        paddq  xmm9,xmm11	;;
%ifidn %3,cnv1
        pxor   xmm1,xmm5	;;
        pxor   xmm9,xmm13	;;
%endif

        movaps [rdi+rsi],xmm1	;;
        pxor   xmm1,xmm4	;;

        movaps [rdi+r11+%1],xmm9 ;; 
        pxor   xmm9,xmm12	;;

        dec    r10d
        movdqa xmm2,xmm0
        movdqa xmm10,xmm8
        jne .0
        pop    rsi
	pop    r15
	pop    r14
	pop    r13
	pop    r12
	pop    rbx
	ret
%endmacro

cnh_mix: defmix 0x400000, 0x40000, cnh
cn_mix_v1_x1: defmix 0x200000, 0x80000, cnv1
cnl_mix_v0_x1: defmix 0x100000, 0x40000, cn
cnl_mix_v0_x2: defmix2 0x100000, 0x40000, cn

%if 0
/*
cnh:
tr -> mix -> im1
ex -> mix -> im1 -> im2

ArenaState { mixable, mixing, splodable, sploding }
StateState { ready, recycling }

6 sploders
24 nt-mixers
(12 sploder-mixers 12 mixers)

*/
%endif

%macro defsplode 1
	push   rbx
	push   rcx
	push   rdx
	;; implode into xmm 0-7
	movaps xmm0,[rcx]
	movaps xmm1,[rcx+0x10]
	movaps xmm2,[rcx+0x20]
	movaps xmm3,[rcx+0x30]
	movaps xmm4,[rcx+0x40]
	movaps xmm5,[rcx+0x50]
	movaps xmm6,[rcx+0x60]
	movaps xmm7,[rcx+0x70]
;;; cnh: extra im pass
%ifidn %1,cnh
align 16
.2:
	pxor   xmm0,[rdx]
	pxor   xmm1,[rdx+0x10]
	pxor   xmm2,[rdx+0x20]
	pxor   xmm3,[rdx+0x30]
	pxor   xmm4,[rdx+0x40]
	pxor   xmm5,[rdx+0x50]
	pxor   xmm6,[rdx+0x60]
	pxor   xmm7,[rdx+0x70]
	xor    eax,eax
.3:
	movaps xmm8,[rdi+rax]
	aesenc xmm0,xmm8
	aesenc xmm1,xmm8
	aesenc xmm2,xmm8
	aesenc xmm3,xmm8
	aesenc xmm4,xmm8
	aesenc xmm5,xmm8
	aesenc xmm6,xmm8
	aesenc xmm7,xmm8
	movdqa xmm8,xmm0
	pxor   xmm0,xmm1
	pxor   xmm1,xmm2
	pxor   xmm2,xmm3
	pxor   xmm3,xmm4
	pxor   xmm4,xmm5
	pxor   xmm5,xmm6
	pxor   xmm6,xmm7
	pxor   xmm7,xmm8
	add    eax,0x10
	cmp    eax,0xa0
	jne .3
	add    rdx,0x80
	cmp    r9,rdx
	jne .2
	pop    rdx
	push   rdx
%endif
	;; explode from xmm 8-15
	movaps xmm8,[r8]
	movaps xmm9,[r8+0x10]
	movaps xmm10,[r8+0x20]
	movaps xmm11,[r8+0x30]
	movaps xmm12,[r8+0x40]
	movaps xmm13,[r8+0x50]
	movaps xmm14,[r8+0x60]
	movaps xmm15,[r8+0x70]
;;; cnh: mix up ex keys
%ifidn %1,cnh
	mov    r10d,16
	movaps [rcx],xmm0
.mixprop_ex:
	xor    eax,eax
.mixprop_ex_round:
	movaps xmm0,[rsi+rax]
	aesenc xmm8,xmm0
	aesenc xmm9,xmm0
	aesenc xmm10,xmm0
	aesenc xmm11,xmm0
	aesenc xmm12,xmm0
	aesenc xmm13,xmm0
	aesenc xmm14,xmm0
	aesenc xmm15,xmm0
	add    eax,0x10
	cmp    eax,0xa0
	jne .mixprop_ex_round
	movdqa xmm0,xmm8
	pxor   xmm8,xmm9
	pxor   xmm9,xmm10
	pxor   xmm10,xmm11
	pxor   xmm11,xmm12
	pxor   xmm12,xmm13
	pxor   xmm13,xmm14
	pxor   xmm14,xmm15
	pxor   xmm15,xmm0
	dec    r10d
	jnz .mixprop_ex
	movaps xmm0,[rcx]
%endif
;;; main transplode
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
	movaps [rdx],xmm8
align 16
.1im:
	movaps xmm8,[rdi+rax]
	aesenc xmm0,xmm8
	aesenc xmm1,xmm8
	aesenc xmm2,xmm8
	aesenc xmm3,xmm8
	aesenc xmm4,xmm8
	aesenc xmm5,xmm8
	aesenc xmm6,xmm8
	aesenc xmm7,xmm8
	add    eax,0x10
	cmp    eax,0xa0
	jne .1im
	movaps xmm8,[rdx]
	movaps [rdx],xmm0
	xor    eax,eax
align 16
.1ex:
	movaps xmm0,[rsi+rax]
	aesenc xmm8,xmm0
	aesenc xmm9,xmm0
	aesenc xmm10,xmm0
	aesenc xmm11,xmm0
	aesenc xmm12,xmm0
	aesenc xmm13,xmm0
	aesenc xmm14,xmm0
	aesenc xmm15,xmm0
	add    eax,0x10
	cmp    eax,0xa0
	jne .1ex
	movaps xmm0,[rdx]
;;; cnh: extra im mixing
%ifidn %1,cnh
	movaps [rdx],xmm0
	pxor   xmm0,xmm1
	pxor   xmm1,xmm2
	pxor   xmm2,xmm3
	pxor   xmm3,xmm4
	pxor   xmm4,xmm5
	pxor   xmm5,xmm6
	pxor   xmm6,xmm7
	pxor   xmm7,[rdx]
%endif
	movaps [rdx+0x00],xmm8
	movaps [rdx+0x10],xmm9
	movaps [rdx+0x20],xmm10
	movaps [rdx+0x30],xmm11
	movaps [rdx+0x40],xmm12
	movaps [rdx+0x50],xmm13
	movaps [rdx+0x60],xmm14
	movaps [rdx+0x70],xmm15
	add    rdx,0x80
	cmp    r9,rdx
	jne .0
;;; cnh: extra im mixing
%ifidn %1,cnh
	mov    r10d,16
.mixprop_im:
	xor    eax,eax
.mixprop_im_round:
	movaps xmm8,[rsi+rax]
	aesenc xmm0,xmm8
	aesenc xmm1,xmm8
	aesenc xmm2,xmm8
	aesenc xmm3,xmm8
	aesenc xmm4,xmm8
	aesenc xmm5,xmm8
	aesenc xmm6,xmm8
	aesenc xmm7,xmm8
	add    eax,0x10
	cmp    eax,0xa0
	jne .mixprop_im_round
	movdqa xmm8,xmm0
	pxor   xmm0,xmm1
	pxor   xmm1,xmm2
	pxor   xmm2,xmm3
	pxor   xmm3,xmm4
	pxor   xmm4,xmm5
	pxor   xmm5,xmm6
	pxor   xmm6,xmm7
	pxor   xmm7,xmm8
	dec    r10d
	jnz .mixprop_im
%endif
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
%endmacro

cn_transplode: defsplode cn
cnh_transplode: defsplode cnh
