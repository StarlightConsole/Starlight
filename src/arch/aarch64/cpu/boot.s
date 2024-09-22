// loads the address of a symbol into a register, relative
.macro ADR_REL register, symbol
	adrp \register, \symbol
	add \register, \register, #:lo12:\symbol
.endm

.macro ADR_ABS register, symbol
	movz \register, #:abs_g3:\symbol
	movk \register, #:abs_g2_nc:\symbol
	movk \register, #:abs_g1_nc:\symbol
	movk \register, #:abs_g0_nc:\symbol
.endm

// fn _start()
.section .text._start

_start:
	// only proceed if the core executes in EL2, park otherwise
	mrs x0, CurrentEL
	cmp x0, {CONST_CURRENTEL_EL2}
	b.ne .L_parking_loop

	// only proceed on the boot core
	mrs x1, MPIDR_EL1
	and x1, x1, {CONST_CORE_ID_MASK}
	ldr x2, BOOT_CORE_ID // provided by bsp/*/cpu.rs
	cmp x1, x2
	b.ne .L_parking_loop

	// this is the boot core

	// init DRAM
	ADR_REL x0, __bss_start
	ADR_REL x1, __bss_end_exclusive

.L_bss_init_loop:
	cmp x0, x1
	b.eq .L_prepare_rust
	stp xzr, xzr, [x0], #16
	b .L_bss_init_loop

// prepare the jump to rust code
.L_prepare_rust:
	// load the base address of the kernel's translation tables
	ldr x0, PHYS_KERNEL_TABLES_BASE_ADDR // provided by bsp/*/memory/mmu.rs

	// load the absolute addresses of the following symbols
	ADR_ABS x1, __boot_core_stack_end_exclusive
	ADR_ABS x2, kernel_init

	// set the stack pointer ensuring EL2 code can use the stack
	ADR_REL x3, __boot_core_stack_end_exclusive
	mov sp, x3

	// get the cpu's timer counter frequency
	ADR_REL x4, ARCH_TIMER_COUNTER_FREQUENCY // provided by aarch64/time.rs
	mrs x5, CNTFRQ_EL0
	cmp x5, xzr
	b.eq .L_parking_loop
	str w5, [x4]

	// jump to rust code, x0, x1 and x2 hold the function arguments provided to _start_rust()
	b _start_rust

// wait for events indefinitely
.L_parking_loop:
	wfe
	b .L_parking_loop

.size _start, . - _start
.type _start, function
.global _start
