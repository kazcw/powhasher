default rel
global cn_mix_v1_x1
global cn_mix_v1xtl_x1
global cn_mix_v1_x2
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

%macro defmix 4			; ArenaSz Iters DoTweak TweakVar
	push   rbp
	push   rbx
        push   rsi
        movaps xmm1,[rsi+0x00]
        movaps xmm2,[rsi+0x10]
        pxor   xmm1,[rsi+0x20]
        pxor   xmm2,[rsi+0x30]
        movq   r8,xmm1
        mov    r10d,%2
        mov    rbx,r8
%ifidn %3,cnv1
        pxor   xmm5,xmm5
        pinsrq xmm5,rdx,1
%ifidn %4,xtl
	mov    eax,0x20
%else
	mov    eax,0x10
%endif
	pxor   xmm7,xmm7
	pinsrb xmm7,eax,11
%endif
align 16
.0:
        and    ebx, %1 - 0x10   ;; 
        movaps xmm0,[rdi+rbx]	;;
        aesenc xmm0,xmm1	;;
        pxor   xmm2,xmm0
%ifidn %4,xtl
	movdqa xmm3,xmm7        ;; x3: 0x20
	movdqa xmm4,xmm7        ;; x4: 0x20
	movdqa xmm6,xmm2        ;; x6: in
	pand   xmm3,xmm2        ;; cc: in & 0x20

        ;; bb: (in << 5) & 0x20 [in.0x1]
        ;; CC: ~(in << 5) & cc [in.0x1, in.0x20 -> 0x20]
	pslld  xmm2,5           ;; x2: in << 5
	pand   xmm4,xmm2        ;; bb: x2 & 0x20
	pandn  xmm2,xmm3        ;; CC: ~x2 & cc

        ;; AA: in ^ CC [in.*]
	pxor   xmm2,xmm6        ;; AA: aa ^ in

        ;; BB: ((~(in >> 1) & bb) ^ 0x20) >> 1 [in.0x1, in.0x40 -> in.0x10]
	psrld  xmm6,1           ;; x6: in >> 1
	pandn  xmm6,xmm4        ;; x6: ~x6 & bb
	pxor   xmm6,xmm7        ;; BB: x6 ^ 0x20
        psrld  xmm6,1
%elifidn %3,cnv1
	movdqa xmm3,xmm7        ;; x3: 0x10
	movdqa xmm4,xmm7        ;; x4: 0x10
	movdqa xmm6,xmm2        ;; x6: in
	pand   xmm3,xmm2        ;; cc: in & 0x10

        ;; bb: (in << 4) & 0x10 [in.0x1]
        ;; CC: ~(in << 4) & cc [in.0x1, in.0x10 -> 0x20]
	pslld  xmm2,4           ;; x2: in << 4
	pand   xmm4,xmm2        ;; bb: x2 & 0x10
	pandn  xmm2,xmm3        ;; CC: ~x2 & cc
	paddq  xmm2,xmm2        ;; aa: CC << 1

        ;; AA: in ^ (CC << 1) [in.*]
	pxor   xmm2,xmm6        ;; AA: aa ^ in

        ;; BB: (~(in >> 1) & bb) ^ 0x10 [in.0x1, in.0x20 -> in.0x10]
	psrld  xmm6,1           ;; x6: in >> 1
	pandn  xmm6,xmm4        ;; x6: ~x6 & bb
	pxor   xmm6,xmm7        ;; BB: x6 ^ 0x10
%endif
        ;; ou: AA ^ BB
	pxor   xmm2,xmm6        ;; ou: AA ^ BB
        movaps [rdi+rbx],xmm2

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
	div    rsi		;; 
	xor    ebx,eax		;; 
	xor    [rbp],rax
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
        pinsrq xmm5,rcx,1
        pxor   xmm13,xmm13
        pinsrq xmm13,r8,1
	mov    eax,0x10
	pxor   xmm7,xmm7
	pinsrb xmm7,eax,11
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
	movdqa xmm3,xmm7
	movdqa xmm4,xmm7
	pand   xmm3,xmm2
	movdqa xmm6,xmm2
	pslld  xmm2,4
	pand   xmm4,xmm2
	pandn  xmm2,xmm3
	paddq  xmm2,xmm2
	pxor   xmm2,xmm6
	psrld  xmm6,1
	pandn  xmm6,xmm4
	pxor   xmm6,xmm7
	pxor   xmm2,xmm6

	movdqa xmm3,xmm7
	movdqa xmm4,xmm7
	pand   xmm3,xmm10
	movdqa xmm6,xmm10
	pslld  xmm10,4
	pand   xmm4,xmm10
	pandn  xmm10,xmm3
	paddq  xmm10,xmm10
	pxor   xmm10,xmm6
	psrld  xmm6,1
	pandn  xmm6,xmm4
	pxor   xmm6,xmm7
	pxor   xmm10,xmm6
%endif
        movaps [rdi+rbx],xmm2
        movaps [rdi+rcx+%1],xmm10
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
        movdqa xmm12,xmm13
        pxor   xmm12,[rdi+r11+%1]
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

cnh_mix: defmix 0x400000, 0x40000, cnh, _
cn_mix_v1_x1: defmix 0x200000, 0x80000, cnv1, _
cn_mix_v1xtl_x1: defmix 0x200000, 0x80000, cnv1, xtl
cn_mix_v1_x2: defmix2 0x200000, 0x80000, cnv1
cnl_mix_v0_x1: defmix 0x100000, 0x40000, cn, _
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
	push   rbp
	push   rbx
	push   rcx
	push   rdx
	mov    rbp,rdx
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
	pxor   xmm0,[rbp]
	pxor   xmm1,[rbp+0x10]
	pxor   xmm2,[rbp+0x20]
	pxor   xmm3,[rbp+0x30]
	pxor   xmm4,[rbp+0x40]
	pxor   xmm5,[rbp+0x50]
	pxor   xmm6,[rbp+0x60]
	pxor   xmm7,[rbp+0x70]
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
	add    rbp,0x80
	cmp    r9,rbp
	jne .2
	pop    rbp
	push   rbp
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
	pxor   xmm0,[rbp]
	pxor   xmm1,[rbp+0x10]
	pxor   xmm2,[rbp+0x20]
	pxor   xmm3,[rbp+0x30]
	pxor   xmm4,[rbp+0x40]
	pxor   xmm5,[rbp+0x50]
	pxor   xmm6,[rbp+0x60]
	pxor   xmm7,[rbp+0x70]
	xor    eax,eax
	movaps [rbp],xmm8
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
	movaps xmm8,[rbp]
	movaps [rbp],xmm0
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
	movaps xmm0,[rbp]
;;; cnh: extra im mixing
%ifidn %1,cnh
	movaps [rbp],xmm0
	pxor   xmm0,xmm1
	pxor   xmm1,xmm2
	pxor   xmm2,xmm3
	pxor   xmm3,xmm4
	pxor   xmm4,xmm5
	pxor   xmm5,xmm6
	pxor   xmm6,xmm7
	pxor   xmm7,[rbp]
%endif
	movaps [rbp+0x00],xmm8
	movaps [rbp+0x10],xmm9
	movaps [rbp+0x20],xmm10
	movaps [rbp+0x30],xmm11
	movaps [rbp+0x40],xmm12
	movaps [rbp+0x50],xmm13
	movaps [rbp+0x60],xmm14
	movaps [rbp+0x70],xmm15
	add    rbp,0x80
	cmp    r9,rbp
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
	pop    rbp
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
