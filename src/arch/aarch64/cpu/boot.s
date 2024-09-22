// loads the address of a symbol into a register, relative
.macro ADR_REL register, symbol
	adrp \register, \symbol
	add \register, \register, #:lo12:\symbol
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
	// set the stack pointer, this ensures that any core in EL2 that needs the stack will work
	ADR_REL x0, __boot_core_stack_end_exclusive
	mov sp, x0

	// get the cpu's timer counter frequency
	ADR_REL x1, ARCH_TIMER_COUNTER_FREQUENCY // provided by aarch64/time.rs
	mrs x2, CNTFRQ_EL0
	cmp x2, xzr
	b.eq .L_parking_loop
	str w2, [x1]

	// jump to rust code, x0 holds the function argument provided to _start_rust
	b _start_rust

// wait for events indefinitely
.L_parking_loop:
	wfe
	b .L_parking_loop

.size _start, . - _start
.type _start, function
.global _start