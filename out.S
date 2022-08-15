	.text
	.file	"<string>"
	.section	.rodata.cst8,"aM",@progbits,8
	.p2align	3
.LCPI0_0:
	.quad	0x4012000000000000
.LCPI0_1:
	.quad	0x4014000000000000
	.text
	.globl	vrkaMain
	.p2align	4, 0x90
	.type	vrkaMain,@function
vrkaMain:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	subq	$16, %rsp
	.cfi_def_cfa_offset 32
	.cfi_offset %rbx, -16
	movw	$9, 14(%rsp)
	movl	$178307594, 10(%rsp)
	movabsq	$strrepr, %rax
	leaq	10(%rsp), %rdi
	callq	*%rax
	movabsq	$fstrs, %rdi
	movabsq	$printf, %rbx
	movq	%rax, %rsi
	xorl	%eax, %eax
	callq	*%rbx
	movabsq	$pow, %rax
	movabsq	$.LCPI0_0, %rcx
	movsd	(%rcx), %xmm0
	movabsq	$.LCPI0_1, %rcx
	movsd	(%rcx), %xmm1
	callq	*%rax
	movabsq	$fstrd, %rdi
	movb	$1, %al
	callq	*%rbx
	xorl	%eax, %eax
	addq	$16, %rsp
	.cfi_def_cfa_offset 16
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end0:
	.size	vrkaMain, .Lfunc_end0-vrkaMain
	.cfi_endproc

	.type	fstrs,@object
	.section	.rodata,"a",@progbits
fstrs:
	.asciz	"%s \n"
	.size	fstrs, 5

	.type	fstrd,@object
fstrd:
	.asciz	"%.10f \n"
	.size	fstrd, 8

	.section	".note.GNU-stack","",@progbits
