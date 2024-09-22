PAGE_SIZE = 64K;
PAGE_MASK = PAGE_SIZE - 1;

__rpi_phys_dram_start_addr = 0;
__rpi_phys_binary_load_addr = 0x80000;

ENTRY(__rpi_phys_binary_load_addr)

PHDRS {
	segment_boot_core_stack PT_LOAD FLAGS(6);
	segment_code PT_LOAD FLAGS(5);
	segment_data PT_LOAD FLAGS(6);
}

SECTIONS {
	. = __rpi_phys_dram_start_addr;

	.boot_core_stack (NOLOAD) : {
		__boot_core_stack_start = .;
		. += __rpi_phys_binary_load_addr;
		__boot_core_stack_end_exclusive = .;
	} :segment_boot_core_stack

	ASSERT((. & PAGE_MASK) == 0, "End of boot core stack is not page aligned")

	__code_start = .;
	.text : {
		KEEP(*(.text._start))
		*(.text._start_arguments) // constants (rust statics) read by _start()
		*(.text._start_rust) // rust entry point
		*(.text*)
	} :segment_code

	.rodata : ALIGN(8) {
		*(.rodata*)
	} :segment_code

	. = ALIGN(PAGE_SIZE);
	__code_end_exclusive = .;

	__data_start = .;
	.data : {
		*(.data*)
	} :segment_data

	.bss (NOLOAD) : ALIGN(16) {
		__bss_start = .;
		*(.bss*);
		. = ALIGN(16);
		__bss_end_exclusive = .;
	} :segment_data

	. = ALIGN(PAGE_SIZE);
	__data_end_exclusive = .;

	__mmio_remap_start = .;
	. += 8 * 1024 * 1024;
	__mmio_remap_end_exclusive = .;

	ASSERT((. & PAGE_MASK) == 0, "MMIO remap reservation is not page aligned")

	.got : {
		*(.got*)
	}

	ASSERT(SIZEOF(.got) == 0, "relocation support not expected")

	/DISCARD/ : {
		*(.comment*)
	}
}
